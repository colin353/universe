use crate::fs;
use pool::ThreadPool;
use std::ffi::OsStr;
use std::sync::Arc;
use time::Timespec;

pub struct WeldParallelFs<C: largetable_client::LargeTableClient> {
    filesystem: Arc<fs::WeldFS<C>>,
    threadpool: ThreadPool,
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
        self.threadpool.execute(move || {
            fs.rmdir(parent, name, reply);
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
        atime: Option<Timespec>,
        mtime: Option<Timespec>,
        fh: Option<u64>,
        crtime: Option<Timespec>,
        chgtime: Option<Timespec>,
        bkuptime: Option<Timespec>,
        flags: Option<u32>,
        reply: fuse::ReplyAttr,
    ) {
        let fs = self.filesystem.clone();
        self.threadpool.execute(move || {
            fs.setattr(
                ino, mode, uid, gid, size, atime, mtime, fh, crtime, chgtime, bkuptime, flags,
                reply,
            );
        });
    }

    fn readdir(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
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
        self.threadpool.execute(move || {
            fs.write(ino, fh, offset, data, flags, reply);
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
        self.threadpool.execute(move || {
            fs.mkdir(parent, name, mode, reply);
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
        self.threadpool.execute(move || {
            fs.rename(parent, name, newparent, newname, reply);
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
        self.threadpool.execute(move || {
            fs.unlink(parent, name, reply);
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
        self.threadpool.execute(move || {
            fs.create(parent, name, mode, flags, reply);
        });
    }
}