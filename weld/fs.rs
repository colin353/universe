use std::cmp;
use std::collections::HashMap;
use std::ffi::OsStr;

use fuse;
use fuse::FileType;
use libc::ENOENT;
use libc::ENOSYS;
use libc::EPERM;
use std::hash;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time;

use weld::File;

const TTL: u64 = 5;

fn file_attr_from_file(ino: u64, file: &File) -> fuse::FileAttr {
    fuse::FileAttr {
        ino: ino,
        size: file.get_size(),
        blocks: 1,
        atime: UNIX_EPOCH + Duration::new(file.get_atime(), 0),
        mtime: UNIX_EPOCH + Duration::new(file.get_mtime(), 0),
        ctime: UNIX_EPOCH + Duration::new(file.get_ctime(), 0),
        crtime: UNIX_EPOCH + Duration::new(file.get_crtime(), 0),
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

fn file_attr_for_symlink(ino: u64) -> fuse::FileAttr {
    fuse::FileAttr {
        ino: ino,
        size: 0,
        blocks: 1,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileType::Symlink,
        perm: 777,
        nlink: 1,
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
    }
}

fn make_dir_attr(ino: u64, size: u64) -> fuse::FileAttr {
    fuse::FileAttr {
        ino: ino,
        size: size,
        blocks: 1,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Symlink {
    ino: u64,
    name: String,
    path: String,
    parent_path: String,
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
    nodes: RwLock<HashMap<u64, (Origin, String)>>,
    paths: RwLock<HashMap<(Origin, String), u64>>,
    symlinks_paths: RwLock<HashMap<(Origin, String), Vec<Symlink>>>,
    symlinks_inos: RwLock<HashMap<u64, Symlink>>,
    next_ino: Mutex<u64>,
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

impl<C: largetable_client::LargeTableClient> WeldFS<C> {
    pub fn new(repo: weld_repo::Repo<C, weld::WeldServerClient>) -> Self {
        let nodes = RwLock::new(HashMap::new());
        let paths = RwLock::new(HashMap::new());

        let mut fs = WeldFS {
            repo: repo,
            nodes: nodes,
            paths: paths,
            next_ino: Mutex::new(1),
            symlinks_inos: RwLock::new(HashMap::new()),
            symlinks_paths: RwLock::new(HashMap::new()),
        };

        fs.link_path_ino(Origin::Root, String::from("/"));
        fs.link_path_ino(Origin::Root, String::from("."));
        fs
    }

    fn link_path_ino(&self, origin: Origin, path: String) -> u64 {
        let ino = {
            let mut ino_mut = self.next_ino.lock().unwrap();
            let ino = ino_mut.clone();
            *ino_mut += 1;
            ino
        };

        self.nodes
            .write()
            .unwrap()
            .insert(ino, (origin.clone(), path.clone()));
        self.paths.write().unwrap().insert((origin, path), ino);

        ino
    }

    fn node_to_path(&self, node: u64) -> Option<(Origin, String)> {
        match self.nodes.read().unwrap().get(&node) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }

    // Get the node for a path, and remember that path for the future.
    fn path_to_node(&self, origin: Origin, path: &str) -> u64 {
        // First, see if this path/node already exists. If so, we can
        // just return it.
        if let Some(ino) = self
            .paths
            .read()
            .unwrap()
            .get(&(origin.clone(), path.to_owned()))
        {
            return *ino;
        }

        self.link_path_ino(origin, path.to_owned())
    }

    fn update_path(&self, origin: Origin, old_path: &str, new_path: &str) -> u64 {
        let ino = if let Some(ino) = self
            .paths
            .read()
            .unwrap()
            .get(&(origin.clone(), old_path.to_owned()))
        {
            *ino
        } else {
            return self.link_path_ino(origin, new_path.to_owned());
        };

        // If the node already exists, we need to update it.
        self.nodes
            .write()
            .unwrap()
            .insert(ino, (origin.clone(), new_path.to_owned()));
        {
            let mut mut_paths = self.paths.write().unwrap();
            // Delete the old path entry.
            mut_paths.remove(&(origin.clone(), old_path.to_owned()));
            // Insert a new path entry with the old ino.
            mut_paths.insert((origin, new_path.to_owned()), ino);
        }
        return ino;
    }

    fn route(&self, parent_ino: u64, filename: &str) -> Option<(Origin, String)> {
        let (origin, parent_path) = match self.node_to_path(parent_ino) {
            Some(x) => x,
            None => {
                return None;
            }
        };

        let assembled_path = format!("{}/{}", parent_path.trim_right_matches('/'), filename,);

        Some((origin, assembled_path))
    }

    fn get_symlink(&self, ino: u64) -> Option<Symlink> {
        match self.symlinks_inos.read().unwrap().get(&ino) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }

    fn list_symlinks(&self, origin: &Origin, parent_path: &str) -> Vec<Symlink> {
        match self
            .symlinks_paths
            .read()
            .unwrap()
            .get(&(origin.to_owned(), parent_path.to_owned()))
        {
            Some(x) => x.to_owned(),
            None => Vec::new(),
        }
    }

    fn create_symlink(
        &self,
        origin: Origin,
        parent_path: String,
        name: String,
        path: String,
    ) -> u64 {
        let ino = {
            let mut ino_mut = self.next_ino.lock().unwrap();
            let ino = ino_mut.clone();
            *ino_mut += 1;
            ino
        };

        let symlink = Symlink {
            ino: ino,
            name: name,
            path: path,
            parent_path: parent_path.clone(),
        };
        self.symlinks_inos
            .write()
            .unwrap()
            .insert(ino, symlink.clone());
        if let Some(ref mut dir) = self
            .symlinks_paths
            .write()
            .unwrap()
            .get_mut(&(origin.clone(), parent_path.clone()))
        {
            dir.push(symlink);
            return ino;
        }
        self.symlinks_paths
            .write()
            .unwrap()
            .insert((origin, parent_path), vec![symlink]);

        ino
    }

    fn delete_symlink(&self, origin: &Origin, ino: u64) {
        let s = match self.symlinks_inos.write().unwrap().remove(&ino) {
            Some(s) => s,
            None => return,
        };
        match self
            .symlinks_paths
            .write()
            .unwrap()
            .get_mut(&(origin.clone(), s.parent_path))
        {
            Some(ref mut dir) => dir.retain(|ref x| x.ino != ino),
            None => return,
        }
    }

    fn readdir_space(&self, id: u64, path: &str) -> Vec<DirectoryListingEntry> {
        //println!("readdir_space");
        let mut entries = Vec::new();

        // Read symlinks
        for symlink in self.list_symlinks(&Origin::from_change(id), &path) {
            entries.push(DirectoryListingEntry::new(
                symlink.ino,
                symlink.name,
                FileType::Symlink,
            ));
        }

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

    fn readdir_root(&self) -> Vec<DirectoryListingEntry> {
        ////println!("readdir_root");
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

    // -------------------------------------------
    // fuse::Filesystem implementation starts ----
    // -------------------------------------------
    pub fn lookup(&self, parent: u64, name: String, reply: fuse::ReplyEntry) {
        let (origin, parent_path) = match self.node_to_path(parent) {
            Some(x) => x,
            None => {
                return reply.error(ENOENT);
            }
        };

        // Check if there's a symlink. If so, quit early.
        for symlink in self.list_symlinks(&origin, &parent_path) {
            if symlink.name == name {
                return reply.entry(
                    &Duration::from_secs(TTL),
                    &file_attr_for_symlink(symlink.ino),
                    0,
                );
            }
        }

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
                return reply.entry(&Duration::from_secs(TTL), &make_dir_attr(ino, 0), 0);
            }
            Origin::Change(id) => {
                let path = format!("{}/{}", parent_path.trim_right_matches('/'), name);

                let ino = self.path_to_node(Origin::from_change(id), &path);

                let file = match self.repo.read_attrs(id, &path, 0) {
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
                    reply.entry(&Duration::from_secs(TTL), &make_dir_attr(ino, 0), 0);
                } else {
                    reply.entry(
                        &Duration::from_secs(TTL),
                        &file_attr_from_file(ino, &file),
                        0,
                    );
                }
            }
        }
    }

    pub fn getattr(&self, ino: u64, reply: fuse::ReplyAttr) {
        // Special case for the root inode, which is always 1.
        if ino == 1 {
            return reply.attr(&Duration::from_secs(TTL), &make_dir_attr(0, 0));
        }

        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                return reply.error(ENOENT);
            }
        };

        // Check if there's a symlink. If so, quit early.
        if let Some(symlink) = self.get_symlink(ino) {
            return reply.attr(
                &Duration::from_secs(TTL),
                &file_attr_for_symlink(symlink.ino),
            );
        }

        // All inodes in the root are just client names.
        match origin {
            Origin::Root => return reply.attr(&Duration::from_secs(TTL), &make_dir_attr(ino, 0)),
            Origin::Change(id) => {
                // The root path isn't written into the database. We have to
                // handle that case as a special case.
                if path == "/" {
                    return reply.attr(&Duration::from_secs(TTL), &make_dir_attr(ino, 0));
                }

                let file = match self.repo.read_attrs(id, &path, 0) {
                    Some(f) => f,
                    None => {
                        return reply.error(ENOENT);
                    }
                };

                if file.get_deleted() {
                    return reply.error(ENOENT);
                }

                match file.get_directory() {
                    true => reply.attr(&Duration::from_secs(TTL), &make_dir_attr(ino, 0)),
                    false => {
                        reply.attr(&Duration::from_secs(TTL), &file_attr_from_file(ino, &file))
                    }
                }
            }
        };
    }

    pub fn read(&self, ino: u64, _fh: u64, offset: i64, size: u32, reply: fuse::ReplyData) {
        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                return reply.error(ENOENT);
            }
        };

        match origin {
            Origin::Root => return reply.error(ENOENT),
            Origin::Change(id) => {
                let file = match self.repo.read(id, &path, 0) {
                    Some(f) => f,
                    None => {
                        return reply.error(ENOENT);
                    }
                };

                if file.get_deleted() || file.get_directory() {
                    return reply.error(ENOENT);
                }

                let start = std::cmp::min(file.get_contents().len(), offset as usize);
                let end = std::cmp::min(file.get_contents().len(), offset as usize + size as usize);
                reply.data(&file.get_contents()[start..end]);
            }
        }
    }

    pub fn rmdir(&self, parent: u64, name: String, reply: fuse::ReplyEmpty) {
        let (origin, path) = match self.route(parent, &name) {
            Some(x) => x,
            None => {
                return reply.error(ENOENT);
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

    pub fn setattr(
        &self,
        ino: u64,
        mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        atime: Option<SystemTime>,
        mtime: Option<SystemTime>,
        _fh: Option<u64>,
        crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: fuse::ReplyAttr,
    ) {
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
                let mut file = match self.repo.read_attrs(id, &path, 0) {
                    Some(f) => f,
                    None => {
                        return reply.error(ENOENT);
                    }
                };

                if file.get_deleted() {
                    return reply.error(ENOENT);
                }

                if let Some(x) = atime {
                    file.set_atime(x.duration_since(UNIX_EPOCH).unwrap().as_secs());
                }
                if let Some(x) = mtime {
                    file.set_mtime(x.duration_since(UNIX_EPOCH).unwrap().as_secs());
                }
                if let Some(x) = crtime {
                    file.set_crtime(x.duration_since(UNIX_EPOCH).unwrap().as_secs());
                }
                if let Some(x) = flags {
                    file.set_flags(x as u64);
                }
                if let Some(x) = mode {
                    file.set_perm(x as u64);
                }

                file.set_filename(path.to_owned());
                reply.attr(&Duration::from_secs(TTL), &file_attr_from_file(ino, &file));
                self.repo.write_attrs(id, file, 0);
            }
        }
    }

    pub fn readdir(&self, ino: u64, _fh: u64, offset: i64, mut reply: fuse::ReplyDirectory) {
        let (origin, path) = match self.node_to_path(ino) {
            Some(x) => x,
            None => {
                return reply.error(ENOENT);
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

    pub fn write(
        &self,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut data: Vec<u8>,
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
                    Some(f) => {
                        if !f.get_deleted() {
                            f
                        } else {
                            make_default_file()
                        }
                    }
                    None => make_default_file(),
                };
                let len = data.len();
                let offset = std::cmp::min(file.get_contents().len(), offset as usize);
                let mut joined_data = Vec::from(&file.get_contents()[0..offset]);
                joined_data.append(&mut data);
                file.set_contents(joined_data);
                file.set_filename(path);
                self.repo.write(id, file, 0);

                reply.written(len as u32);
            }
        }
    }

    pub fn mkdir(&self, parent: u64, name: String, _mode: u32, reply: fuse::ReplyEntry) {
        let (origin, path) = match self.route(parent, &name) {
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
                reply.entry(&Duration::from_secs(TTL), &make_dir_attr(ino, 0), 0);
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
        println!("getlk");
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
        println!("getlk");
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
        println!("link");
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
        println!("flush");
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
        println!("release");
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
        println!("fsync");
        reply.ok()
    }

    pub fn rename(
        &self,
        parent: u64,
        name: String,
        newparent: u64,
        newname: String,
        reply: fuse::ReplyEmpty,
    ) {
        let (source_origin, source_path) = match self.route(parent, &name) {
            Some(x) => x,
            None => return reply.error(ENOENT),
        };

        let (dest_origin, dest_path) = match self.route(newparent, &newname) {
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

    pub fn open(&self, ino: u64, flags: u32, reply: fuse::ReplyOpen) {
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
                let file = match self.repo.read_attrs(id, &path, 0) {
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

    pub fn unlink(&self, parent: u64, name: String, reply: fuse::ReplyEmpty) {
        //println!("unlink: {:?} within {}", name, parent);

        let (origin, path) = match self.route(parent, &name) {
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

    pub fn create(
        &self,
        parent: u64,
        name: String,
        _mode: u32,
        _flags: u32,
        reply: fuse::ReplyCreate,
    ) {
        let (origin, path) = match self.route(parent, &name) {
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

                reply.created(
                    &Duration::from_secs(TTL),
                    &file_attr_from_file(ino, &file),
                    0,
                    ino,
                    _flags,
                );
                self.repo.write(id, file, 0);
            }
        }
    }

    pub fn symlink(
        &self,
        parent: u64,
        name: String,
        link: std::path::PathBuf,
        reply: fuse::ReplyEntry,
    ) {
        let (origin, parent_path) = match self.node_to_path(parent) {
            Some(x) => x,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let ino = self.create_symlink(
            origin,
            parent_path,
            name,
            link.to_string_lossy().to_string(),
        );

        reply.entry(&Duration::from_secs(TTL), &file_attr_for_symlink(ino), 0);
    }

    pub fn readlink(&self, ino: u64, reply: fuse::ReplyData) {
        match self.get_symlink(ino) {
            Some(symlink) => reply.data(&symlink.path.as_bytes()),
            None => reply.error(ENOENT),
        }
    }
}
