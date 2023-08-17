pub struct Metadata<'a> {
    table: sstable::SSTableReader<service::FileView<'a>>,
}

type Filter = String;

impl<'a> Metadata<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (String, service::FileView)> {
        self.table.iter()
    }

    pub fn key(&self, path: &str) -> String {
        format!("{:03}/{}", path.split("/").count(), path)
    }

    pub fn filter_key(&self, path: &str) -> Filter {
        if path.is_empty() {
            "001/".to_string()
        } else {
            format!("{:03}/{}/", path.split("/").count() + 1, path)
        }
    }

    pub fn list_directory(
        &'a self,
        filter: &'a Filter,
    ) -> impl Iterator<Item = (String, service::FileView)> {
        self.table
            .iter_at(sstable::Filter {
                spec: &filter,
                min: "",
                max: "",
            })
            .map(|(s, f)| {
                let first_slash = s.find('/').map(|i| i + 1).unwrap_or(0);
                (s[first_slash..].to_owned(), f)
            })
    }

    pub fn empty() -> Self {
        Self {
            table: sstable::SSTableReader::empty(),
        }
    }

    pub fn from_path(path: std::path::PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            table: sstable::SSTableReader::from_filename(path)?,
        })
    }

    pub fn get(&self, path: &str) -> Option<service::FileView> {
        self.table.get(&self.key(path))
    }

    pub fn diff(&self, other: &'a Metadata<'a>) -> MetadataDiffIterator {
        MetadataDiffIterator {
            left: self.table.iter().peekable(),
            right: other.table.iter().peekable(),
        }
    }
}

pub struct MetadataDiffIterator<'a> {
    left: std::iter::Peekable<sstable::SSTableIterator<'a, service::FileView<'a>>>,
    right: std::iter::Peekable<sstable::SSTableIterator<'a, service::FileView<'a>>>,
}

impl<'a> Iterator for MetadataDiffIterator<'a> {
    type Item = (
        String,
        Option<service::FileView<'a>>,
        Option<service::FileView<'a>>,
    );
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (self.left.peek(), self.right.peek()) {
                (Some((lp, lf)), Some((rp, rf))) => {
                    if lp == rp {
                        let (lp, lf) = self.left.next().unwrap();
                        let (_, rf) = self.right.next().unwrap();

                        // Skip if no changes
                        if lf.get_is_dir() == rf.get_is_dir()
                            && lf.get_sha() == rf.get_sha()
                            && lf.get_mtime() == rf.get_mtime()
                            && lf.get_length() == rf.get_length()
                        {
                            continue;
                        }
                        return Some((lp, Some(lf), Some(rf)));
                    } else if lp < rp {
                        let (lp, lf) = self.left.next().unwrap();
                        return Some((lp, Some(lf), None));
                    } else {
                        let (rp, rf) = self.right.next().unwrap();
                        return Some((rp, None, Some(rf)));
                    }
                }
                (Some(_), None) => {
                    let (lp, lf) = self.left.next().unwrap();
                    return Some((lp, Some(lf), None));
                }
                (None, Some(_)) => {
                    let (rp, rf) = self.right.next().unwrap();
                    return Some((rp, None, Some(rf)));
                }
                (None, None) => return None,
            }
        }
    }
}
