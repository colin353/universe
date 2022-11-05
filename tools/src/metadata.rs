pub struct Metadata<'a> {
    table: sstable::SSTableReader<service::FileView<'a>>,
}

impl<'a> Metadata<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (String, service::FileView)> {
        self.table.iter()
    }

    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        Ok(Self {
            table: sstable::SSTableReader::from_filename(path)?,
        })
    }

    pub fn get(&self, path: &str) -> Option<service::FileView> {
        let key = format!("{}/{}", path.split("/").count(), path);
        self.table.get(&key)
    }
}
