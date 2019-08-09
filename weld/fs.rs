use std::cmp;
use std::collections::HashMap;
use std::ffi::OsStr;

use fuse;
use fuse::FileType;
use libc::ENOENT;
use libc::ENOSYS;
use libc::EPERM;
use std::hash;
use time;
use time::Timespec;

use weld::File;

const TTL: Timespec = Timespec { sec: 1, nsec: 0 }; // 10 seconds

const CREATE_TIME: Timespec = Timespec {
    sec: 1381237736,
    nsec: 0,
}; // 2013-10-08 08:56

fn file_attr_from_file(ino: u64, file: &File) -> fuse::FileAttr {
    fuse::FileAttr {
        ino: ino,
        size: file.get_contents().len() as u64,
        blocks: 1,
        atime: Timespec::new(file.get_atime() as i64, 0),
        mtime: Timespec::new(file.get_mtime() as i64, 0),
        ctime: Timespec::new(file.get_ctime() as i64, 0),
        crtime: Timespec::new(file.get_crtime() as i64, 0),
        kind: if file.get_directory() {
            FileType::Directory
        } else {
            FileType::RegularFile
        },
        perm: file.get_perm() as u16,
        nlink: file.get_nlink() as u32,
        uid: 1000,
        gid: 1000,
        rdev: file.get_rdev() as u32,
        flags: file.get_flags() as u32,
    }
}

fn make_dir_attr(ino: u64, size: u64) -> fuse::FileAttr {
    fuse::FileAttr {
        ino: ino,
        size: size,
        blocks: 1,
        atime: CREATE_TIME,
        mtime: CREATE_TIME,
        ctime: CREATE_TIME,
        crtime: CREATE_TIME,
        kind: FileType::Directory,
        perm: 0o644,
        nlink: 1,
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
    }
}

fn make_default_file() -> File {
    let mut file = File::new();
    file.set_perm(0o666);

    let ts = time::now().to_timespec().sec as u64;
    file.set_atime(ts);
    file.set_ctime(ts);
    file.set_crtime(ts);
    file.set_mtime(ts);

    file
}

#[derive(Clone, Debug, Eq)]
enum Origin {
    Change(u64),
    Root,
}

impl Origin {
    pub fn from_change(id: u64) -> Origin {
        Origin::Change(id)
    }
}

impl hash::Hash for Origin {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match &self {
            Origin::Change(id) => {
                4321.hash(state);
                id.hash(state);
            }
            Origin::Root => {
                1234.hash(state);
            }
        }
    }
}

impl cmp::PartialEq for Origin {
    fn eq(&self, other: &Origin) -> bool {
        match (&self, other) {
            (Origin::Change(x), Origin::Change(y)) => (x == y),
            (Origin::Root, Origin::Root) => true,
            _ => false,
        }
    }
}

struct DirectoryListingEntry {
    inode: u64,
    name: String,
    filetype: FileType,
}

impl DirectoryListingEntry {
    pub fn new(inode: u64, name: String, filetype: FileType) -> Self {
        DirectoryListingEntry {
            inode: inode,
            name: name,
            filetype: filetype,
        }
    }
}

pub struct WeldFS<C: largetable_client::LargeTableClient> {
    repo: weld_repo::Repo<C, weld::WeldServerClient>,
    nodes: HashMap<u64, (Origin, String)>,
    paths: HashMap<(Origin, String), u64>,
    next_ino: u64,
}

impl<C: largetable_client::LargeTableClient> WeldFS<C> {
    pub fn new(repo: weld_repo::Repo<C, weld::WeldServerClient>) -> Self {
        let nodes = HashMap::new();
        let paths = HashMap::new();

        let mut fs = WeldFS {
            repo: repo,
            nodes: nodes,
            paths: paths,
            next_ino: 1,
        };

        fs.link_path_ino(Origin::Root, String::from("/"));
        fs.link_path_ino(Origin::Root, String::from("."));
        fs
    }

    fn link_path_ino(&mut self, origin: Origin, path: String) -> u64 {
        let ino = self.next_ino;
        self.next_ino += 1;

        self.nodes.insert(ino, (origin.clone(), path.clone()));
        self.paths.insert((origin, path), ino);

        ino
    }

    fn node_to_path(&self, node: u64) -> Option<(Origin, String)> {
        match self.nodes.get(&node) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }

    // Get the node for a path, and remember that path for the future.
    fn path_to_node(&mut self, origin: Origin, path: &str) -> u64 {
        // First, see if this path/node already exists. If so, we can
        // just return it.
        if let Some(ino) = self.paths.get(&(origin.clone(), path.to_owned())) {
            return *ino;
        }

        self.link_path_ino(origin, path.to_owned())
    }

    fn update_path(&mut self, origin: Origin, old_path: &str, new_path: &str) -> u64 {
        match self.paths.get(&(origin.clone(), old_path.to_owned())) {
            Some(&ino) => {
                // If the node already exists, we need to update it.
                self.nodes
                    .insert(ino, (origin.clone(), new_path.to_owned()));
                // Delete the old path entry.
                self.paths.remove(&(origin.clone(), old_path.to_owned()));
                // Insert a new path entry with the old ino.
                self.paths.insert((origin, new_path.to_owned()), ino);
                ino
            }
            None => self.link_path_ino(origin, new_path.to_owned()),
        }
    }

    fn route(&self, parent_ino: u64, filename: &OsStr) -> Option<(Origin, String)> {
        let (origin, parent_path) = match self.node_to_path(parent_ino) {
            Some(x) => x,
            None => {
                return None;
            }
        };

        let assembled_path = format!(
            "{}/{}",
            parent_path.trim_right_matches('/'),
            filename.to_string_lossy()
        );

        Some((origin, assembled_path))
    }

    fn readdir_space(&mut self, id: u64, path: &str) -> Vec<DirectoryListingEntry> {
        let mut entries = Vec::new();

        for file in self.repo.list_files(id, &path, 0) {
            let node = self.path_to_node(Origin::from_change(id), file.get_filename());
            let filename: String;
            {
                filename = path_to_filename(file.get_filename()).to_owned()
            };
            let filetype = match file.get_directory() {
                true => FileType::Directory,
                false => FileType::RegularFile,
            };
            entries.push(DirectoryListingEntry::new(node, filename, filetype));
        }

        entries
    }

    fn readdir_root(&mut self) -> Vec<DirectoryListingEntry> {
        let changes = self.repo.list_changes().collect::<Vec<_>>();

        changes
            .iter()
            .map(|change| {
                let node = self.path_to_node(Origin::Root, change.get_friendly_name());
                DirectoryListingEntry::new(
                    node,
                    change.get_friendly_name().to_owned(),
                    FileType::Directory,
                )
            })
            .collect()
    }
}

pub fn path_to_filename(path: &str) -> String {
    let is_directory = path.rfind("/") == Some(path.len() - 1);
    let mut iter = path.rsplit("/");

    match is_directory {
        true => {
            iter.next();
            let fname = match iter.next() {
                Some(x) => x,
                None => "",
            };
            format!("{}/", fname)
        }
        false => match iter.next() {
            Some(x) => x.to_owned(),
            None => String::from(""),
        },
    }
}

impl<C: largetable_client::LargeTableClient> fuse::Filesystem for WeldFS<C> {
    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        //println!("lookup: {} -> {:?}", parent, name);
        let (origin, parent_path) = match self.node_to_path(parent) {
            Some(x) => x,
            None => {
                return reply.error(ENOENT);
            }
        };

        let name = name.to_string_lossy().to_string();

        match origin {
            Origin::Root => {
                let id: u64 = match self.repo.lookup_friendly_name(&name) {
                    Some(id) => id,
                    None => return reply.error(ENOENT),
                };

                if let None = self.repo.get_change(id) {
                    return reply.error(ENOENT);
                }

                let ino = self.path_to_node(Origin::from_change(id), "/");
                return reply.entry(&TTL, &make_dir_attr(ino, 0), 0);
            }
            Origin::Change(id) => {
                let path = format!("{}/{}", parent_path.trim_right_matches('/'), name);

                let ino = self.path_to_node(Origin::from_change(id), &path);

                let file = match self.repo.read(id, &path, 0) {
                    Some(f) => f,
                    None => {
                        return reply.error(ENOENT);
                    }
                };

                // If the file was deleted, return ENOENT.
                if file.get_deleted() {
                    return reply.error(ENOENT);
                }

                if file.get_directory() {
                    reply.entry(&TTL, &make_dir_attr(ino, 0), 0);
                } else {
                    reply.entry(&TTL, &file_attr_from_file(ino, &file), 0);
                }
            }
        }
    }

    fn getattr(&mut self, _req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        //println!("getattr: {}", ino);
        // Special case for the root inode, which is always 1.
        if ino == 1 {
            reply.attr(&TTL, &make_dir_attr(0, 0));
            return;
        }

        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        //println!("extracted path: {}", path);

        // All inodes in the root are just client names.
        match origin {
            Origin::Root => return reply.attr(&TTL, &make_dir_attr(ino, 0)),
            Origin::Change(id) => {
                // The root path isn't written into the database. We have to
                // handle that case as a special case.
                if path == "/" {
                    reply.attr(&TTL, &make_dir_attr(ino, 0));
                    return;
                }

                let file = match self.repo.read(id, &path, 0) {
                    Some(f) => f,
                    None => {
                        //println!("no entry for {} in client {}", path, space);
                        return reply.error(ENOENT);
                    }
                };

                if file.get_deleted() {
                    return reply.error(ENOENT);
                }

                //println!("directory={}", file.get_directory());

                match file.get_directory() {
                    true => reply.attr(&TTL, &make_dir_attr(ino, 0)),
                    false => reply.attr(&TTL, &file_attr_from_file(ino, &file)),
                }
            }
        };
    }

    fn read(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: fuse::ReplyData,
    ) {
        //println!("read: {}, offset={}, size={}", ino, offset, _size);
        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        match origin {
            Origin::Root => return reply.error(ENOENT),
            Origin::Change(id) => {
                let file = match self.repo.read(id, &path, 0) {
                    Some(f) => f,
                    None => {
                        reply.error(ENOENT);
                        return;
                    }
                };

                if file.get_deleted() || file.get_directory() {
                    reply.error(ENOENT);
                    return;
                }

                let start = std::cmp::min(file.get_contents().len(), offset as usize);
                let end = std::cmp::min(file.get_contents().len(), offset as usize + size as usize);
                reply.data(&file.get_contents()[start..end]);
            }
        }
    }

    fn rmdir(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEmpty) {
        //println!("rmdir: {}", name.to_string_lossy());
        let (origin, path) = match self.route(parent, name) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        match origin {
            Origin::Root => reply.error(ENOSYS),
            Origin::Change(id) => {
                self.repo.delete(id, &path, 0);
                reply.ok();
            }
        };
    }

    fn setattr(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        atime: Option<Timespec>,
        mtime: Option<Timespec>,
        _fh: Option<u64>,
        crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        flags: Option<u32>,
        reply: fuse::ReplyAttr,
    ) {
        //println!("setattr: {}", ino);
        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        match origin {
            Origin::Root => return reply.error(ENOENT),
            Origin::Change(id) => {
                let mut file = match self.repo.read(id, &path, 0) {
                    Some(f) => f,
                    None => {
                        reply.error(ENOENT);
                        return;
                    }
                };

                if file.get_deleted() || file.get_directory() {
                    reply.error(ENOENT);
                    return;
                }

                if let Some(x) = atime {
                    file.set_atime(x.sec as u64);
                }
                if let Some(x) = mtime {
                    file.set_mtime(x.sec as u64);
                }
                if let Some(x) = crtime {
                    file.set_crtime(x.sec as u64);
                }
                if let Some(x) = flags {
                    file.set_flags(x as u64);
                }
                if let Some(x) = mode {
                    file.set_perm(x as u64);
                }

                file.set_filename(path.to_owned());
                reply.attr(&TTL, &file_attr_from_file(ino, &file));
                self.repo.write(id, file, 0);
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        //println!("readdir: {}", ino);
        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let mut entries = vec![
            DirectoryListingEntry::new(1, String::from("."), FileType::Directory),
            DirectoryListingEntry::new(1, String::from(".."), FileType::Directory),
        ];

        entries.append(&mut match origin {
            Origin::Root => self.readdir_root(),
            Origin::Change(id) => self.readdir_space(id, &path),
        });

        let to_skip = if offset == 0 { offset } else { offset + 1 } as usize;
        for (i, entry) in entries.into_iter().enumerate().skip(to_skip) {
            reply.add(entry.inode, i as i64, entry.filetype, entry.name);
        }
        reply.ok();
    }

    fn write(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        _offset: i64,
        data: &[u8],
        _flags: u32,
        reply: fuse::ReplyWrite,
    ) {
        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        match origin {
            Origin::Root => reply.error(ENOENT),
            Origin::Change(id) => {
                let mut file = match self.repo.read(id, &path, 0) {
                    Some(f) => f,
                    None => make_default_file(),
                };
                file.set_contents(data.to_owned());
                file.set_filename(path);
                self.repo.write(id, file, 0);

                reply.written(data.len() as u32);
            }
        }
    }

    fn mkdir(
        &mut self,
        _req: &fuse::Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        reply: fuse::ReplyEntry,
    ) {
        let (origin, path) = match self.route(parent, name) {
            Some(x) => x,
            None => return reply.error(ENOENT),
        };

        match origin {
            Origin::Root => return reply.error(ENOSYS),
            Origin::Change(id) => {
                let mut dir = File::new();
                dir.set_directory(true);
                dir.set_filename(path.to_owned());

                self.repo.write(id, dir, 0);
                let ino = self.path_to_node(Origin::from_change(id), &path);
                reply.entry(&TTL, &make_dir_attr(ino, 0), 0);
            }
        };
    }

    fn getlk(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: u32,
        _pid: u32,
        reply: fuse::ReplyLock,
    ) {
        reply.error(ENOSYS);
    }

    fn setlk(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: u32,
        _pid: u32,
        _sleep: bool,
        reply: fuse::ReplyEmpty,
    ) {
        reply.error(ENOSYS)
    }

    fn link(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _newparent: u64,
        _newname: &OsStr,
        reply: fuse::ReplyEntry,
    ) {
        reply.error(EPERM)
    }

    fn symlink(
        &mut self,
        _req: &fuse::Request,
        _parent: u64,
        _name: &OsStr,
        _link: &std::path::Path,
        reply: fuse::ReplyEntry,
    ) {
        reply.error(EPERM)
    }

    fn flush(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: fuse::ReplyEmpty,
    ) {
        reply.ok()
    }

    fn release(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: fuse::ReplyEmpty,
    ) {
        reply.ok()
    }

    fn fsync(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: fuse::ReplyEmpty,
    ) {
        reply.ok()
    }

    fn rename(
        &mut self,
        _req: &fuse::Request,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        reply: fuse::ReplyEmpty,
    ) {
        let (source_origin, source_path) = match self.route(parent, name) {
            Some(x) => x,
            None => return reply.error(ENOENT),
        };

        let (dest_origin, dest_path) = match self.route(newparent, newname) {
            Some(x) => x,
            None => return reply.error(ENOENT),
        };

        match (source_origin, dest_origin) {
            (Origin::Change(source_id), Origin::Change(dest_id)) => {
                if source_id != dest_id {
                    return reply.error(ENOENT);
                }

                let mut file = match self.repo.read(source_id, &source_path, 0) {
                    Some(f) => f,
                    None => return reply.error(ENOENT),
                };
                self.repo.delete(source_id, &source_path, 0);

                self.path_to_node(Origin::from_change(source_id), &dest_path);
                self.update_path(Origin::from_change(source_id), &source_path, &dest_path);
                file.set_filename(dest_path.clone());
                self.repo.write(source_id, file, 0);
            }
            _ => return reply.error(ENOENT),
        }

        reply.ok();
    }

    fn open(&mut self, _req: &fuse::Request, ino: u64, flags: u32, reply: fuse::ReplyOpen) {
        //println!("open: ino={} flags={}", ino, flags);
        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        match origin {
            Origin::Root => return reply.error(ENOENT),
            Origin::Change(id) => {
                let file = match self.repo.read(id, &path, 0) {
                    Some(f) => f,
                    None => return reply.error(ENOENT),
                };
                if file.get_deleted() {
                    return reply.error(ENOENT);
                }
            }
        };

        reply.opened(ino, flags);
    }

    fn unlink(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEmpty) {
        //println!("unlink: {:?} within {}", name, parent);

        let (origin, path) = match self.route(parent, name) {
            Some(x) => x,
            None => return reply.error(ENOENT),
        };

        match origin {
            Origin::Root => return reply.error(ENOSYS),
            Origin::Change(id) => {
                //println!("deleting: {}", path);
                self.repo.delete(id, &path, 0);
            }
        }
        reply.ok();
    }

    fn create(
        &mut self,
        _req: &fuse::Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _flags: u32,
        reply: fuse::ReplyCreate,
    ) {
        //println!("create: {:?} within {}", name, parent);
        let (origin, path) = match self.route(parent, name) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        match origin {
            Origin::Root => return reply.error(ENOSYS),
            Origin::Change(id) => {
                let mut file = make_default_file();
                file.set_filename(path.to_owned());

                let ino = self.path_to_node(Origin::from_change(id), &path);

                //println!("\tas {}", ino);
                reply.created(&TTL, &file_attr_from_file(ino, &file), 0, ino, _flags);
                self.repo.write(id, file, 0);
            }
        }
    }
}
