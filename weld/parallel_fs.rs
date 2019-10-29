use crate::fs;
use pool::ThreadPool;
use std::ffi::OsStr;
use std::sync::Arc;
use std::time::SystemTime;

pub struct WeldParallelFs<C: largetable_client::LargeTableClient> {
    filesystem: Arc<fs::WeldFS<C>>,
    threadpool: ThreadPool,
}

fn get_timestamp_usec() -> u64 {
    let tm = time::now_utc().to_timespec();
    (tm.sec as u64) * 1_000_000 + ((tm.nsec / 1000) as u64)
}

impl<C: largetable_client::LargeTableClient> WeldParallelFs<C>
where
    C: Send + Sync,
{
    pub fn new(repo: weld_repo::Repo<C, weld::WeldServerClient>) -> Self {
        WeldParallelFs {
            filesystem: Arc::new(fs::WeldFS::new(repo)),
            threadpool: ThreadPool::new(32),
        }
    }
}

impl<C: largetable_client::LargeTableClient> fuse::Filesystem for WeldParallelFs<C>
where
    C: Send + Sync + 'static,
{
    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        let name = name.to_string_lossy().to_string();
        let fs = self.filesystem.clone();
        self.threadpool.execute(move || {
            fs.lookup(parent, name, reply);
        });
    }

    fn getattr(&mut self, _req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        let fs = self.filesystem.clone();
        self.threadpool.execute(move || {
            fs.getattr(ino, reply);
        });
    }

    fn read(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        reply: fuse::ReplyData,
    ) {
        let fs = self.filesystem.clone();
        self.threadpool.execute(move || {
            fs.read(ino, fh, offset, size, reply);
        });
    }

    fn rmdir(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEmpty) {
        let name = name.to_string_lossy().to_string();
        let fs = self.filesystem.clone();
        let timestamp = get_timestamp_usec();
        self.threadpool.execute(move || {
            fs.rmdir(parent, name, timestamp, reply);
        });
    }

    fn setattr(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<SystemTime>,
        mtime: Option<SystemTime>,
        fh: Option<u64>,
        crtime: Option<SystemTime>,
        chgtime: Option<SystemTime>,
        bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: fuse::ReplyAttr,
    ) {
        let fs = self.filesystem.clone();
        let timestamp = get_timestamp_usec();
        self.threadpool.execute(move || {
            fs.setattr(
                ino, mode, uid, gid, size, atime, mtime, fh, crtime, chgtime, bkuptime, flags,
                timestamp, reply,
            );
        });
    }

    fn readdir(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: fuse::ReplyDirectory,
    ) {
        let fs = self.filesystem.clone();
        self.threadpool.execute(move || {
            fs.readdir(ino, fh, offset, reply);
        });
    }

    fn write(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        flags: u32,
        reply: fuse::ReplyWrite,
    ) {
        let fs = self.filesystem.clone();
        let data = data.to_owned();
        let timestamp = get_timestamp_usec();
        self.threadpool.execute(move || {
            fs.write(ino, fh, offset, data, flags, timestamp, reply);
        });
    }

    fn mkdir(
        &mut self,
        _req: &fuse::Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        reply: fuse::ReplyEntry,
    ) {
        let fs = self.filesystem.clone();
        let name = name.to_string_lossy().to_string();
        let timestamp = get_timestamp_usec();
        self.threadpool.execute(move || {
            fs.mkdir(parent, name, mode, timestamp, reply);
        });
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
        let fs = self.filesystem.clone();
        let name = name.to_string_lossy().to_string();
        let newname = newname.to_string_lossy().to_string();
        let timestamp = get_timestamp_usec();
        self.threadpool.execute(move || {
            fs.rename(parent, name, newparent, newname, timestamp, reply);
        });
    }

    fn open(&mut self, _req: &fuse::Request, ino: u64, flags: u32, reply: fuse::ReplyOpen) {
        let fs = self.filesystem.clone();
        self.threadpool.execute(move || {
            fs.open(ino, flags, reply);
        });
    }

    fn unlink(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEmpty) {
        let fs = self.filesystem.clone();
        let name = name.to_string_lossy().to_string();
        let timestamp = get_timestamp_usec();
        self.threadpool.execute(move || {
            fs.unlink(parent, name, timestamp, reply);
        });
    }

    fn create(
        &mut self,
        _req: &fuse::Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        flags: u32,
        reply: fuse::ReplyCreate,
    ) {
        let fs = self.filesystem.clone();
        let name = name.to_string_lossy().to_string();
        let timestamp = get_timestamp_usec();
        self.threadpool.execute(move || {
            fs.create(parent, name, mode, flags, timestamp, reply);
        });
    }

    fn symlink(
        &mut self,
        _req: &fuse::Request,
        parent: u64,
        name: &OsStr,
        link: &std::path::Path,
        reply: fuse::ReplyEntry,
    ) {
        let fs = self.filesystem.clone();
        let name = name.to_string_lossy().to_string();
        let link = link.to_owned();
        self.threadpool.execute(move || {
            fs.symlink(parent, name, link, reply);
        });
    }

    fn readlink(&mut self, _req: &fuse::Request, ino: u64, reply: fuse::ReplyData) {
        let fs = self.filesystem.clone();
        self.threadpool.execute(move || {
            fs.readlink(ino, reply);
        });
    }
}
