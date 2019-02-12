extern crate largetable_client;
extern crate weld;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use weld::File;

const CHANGES: &'static str = "changes";
const CHANGE_METADATA: &'static str = "metadata";
const CHANGE_IDS: &'static str = "change_ids";
const SNAPSHOTS: &'static str = "snapshots";
const SNAPSHOT_IDS: &'static str = "snapshots_ids";

#[derive(Clone)]
pub struct Repo<C: largetable_client::LargeTableClient, W: weld::WeldServer> {
    db: C,
    remote_server: Option<W>,

    // Map of friendly name to support change lookup by friendly name.
    spaces: Arc<RwLock<HashMap<String, u64>>>,
}

impl<C: largetable_client::LargeTableClient, W: weld::WeldServer> Repo<C, W> {
    pub fn new(client: C) -> Self {
        let mut repo = Repo {
            db: client,
            remote_server: None,
            spaces: Arc::new(RwLock::new(HashMap::new())),
        };

        repo.initialize_space_map();

        repo
    }

    fn initialize_space_map(&mut self) {
        for change in self.list_changes().collect::<Vec<_>>() {
            self.spaces
                .write()
                .unwrap()
                .insert(change.get_friendly_name().to_owned(), change.get_id());
        }
    }

    pub fn lookup_friendly_name(&self, friendly_name: &str) -> Option<u64> {
        match self.spaces.read().unwrap().get(friendly_name) {
            Some(id) => Some(*id),
            None => None,
        }
    }

    pub fn add_remote_server(&mut self, client: W) {
        self.remote_server = Some(client);
    }

    pub fn read_remote(&self, id: u64, path: &str, index: u64) -> Option<File> {
        let filename = normalize_filename(path);

        match self.remote_server {
            Some(ref client) => {
                let mut ident = weld::FileIdentifier::new();
                ident.set_id(id);
                ident.set_filename(filename);
                ident.set_index(index);
                let file = client.read(ident);
                match file.get_found() {
                    true => Some(file),
                    false => None,
                }
            }
            // If we don't have a connected remote server, return nothing.
            None => None,
        }
    }

    pub fn read(&self, id: u64, path: &str, index: u64) -> Option<File> {
        let filename = normalize_filename(path);

        let change = match self.get_change(id) {
            Some(c) => c,
            None => return None,
        };

        // If the current change has a copy of the file, it must be the latest, so return it.
        if let Some(mut file) = self.db.read_proto::<File>(
            &change_to_rowname(id),
            path_to_colname(&filename).as_str(),
            index,
        ) {
            file.set_found(true);
            return Some(file);
        }

        // Otherwise we can fall back to the based change, if it exists.
        if change.get_is_based_locally() {
            self.read(change.get_based_id(), &filename, change.get_based_index())
        } else {
            self.read_remote(change.get_based_id(), &filename, change.get_based_index())
        }
    }

    pub fn write(&self, id: u64, mut file: File, index: u64) {
        // Create the associated parent directories.
        let mut directory = parent_directory(file.get_filename());
        while directory != "/" {
            self.create_directory(id, &directory, index);
            directory = parent_directory(&directory);
        }

        // Later, when the file is read, we should make sure we return
        // true for file.found.
        file.set_found(true);

        self.db.write_proto(
            change_to_rowname(id).as_str(),
            path_to_colname(file.get_filename()).as_str(),
            index,
            &file,
        );
    }

    pub fn delete(&self, id: u64, path: &str, index: u64) {
        let mut file = File::new();
        file.set_filename(path.to_owned());
        file.set_deleted(true);
        self.write(id, file, index)
    }

    pub fn create_directory(&self, id: u64, path: &str, index: u64) {
        // Check if the directory exists. If so, no work required.
        if self.read(id, path, index).is_some() {
            return;
        }

        let mut dir = File::new();
        dir.set_filename(path.to_owned());
        dir.set_directory(true);
        dir.set_found(true);

        self.db.write_proto(
            &change_to_rowname(id).as_str(),
            path_to_colname(path).as_str(),
            index,
            &dir,
        );
    }

    pub fn initialize_head(&mut self, id: u64) {
        self.db.write_proto(
            CHANGE_METADATA,
            &change_to_rowname(id),
            0,
            &weld::Change::new(),
        );
    }

    pub fn make_change(&self, mut change: weld::Change) -> u64 {
        // Reserve a local ID for this change.
        change.set_id(self.reserve_change_id());
        change.set_last_modified_timestamp(weld::get_timestamp_usec());

        // If based_space is empty and index default, and we are connected to a remote server, base
        // this on the remote server latest change ID.
        if !change.get_is_based_locally() && change.get_based_index() == 0 {
            if let Some(ref client) = self.remote_server {
                let latest_change = client.get_latest_change();
                change.set_based_id(0); // based on HEAD.
                change.set_based_index(latest_change.get_id());
            }
        }

        change.set_found(true);
        self.db.write_proto(
            CHANGE_METADATA,
            &change_to_rowname(change.get_id()),
            0,
            &change,
        );

        // Create an initial entry in the snapshots record.
        let mut entry = weld::SnapshotLogEntry::new();
        entry.set_is_rebase(true);
        entry.set_based_id(change.get_based_id());
        entry.set_based_index(change.get_based_index());
        self.log_snapshot(change.get_id(), entry);

        // Update the friendly name mapping.
        self.spaces
            .write()
            .unwrap()
            .insert(change.get_friendly_name().to_owned(), change.get_id());

        change.get_id()
    }

    fn log_snapshot(&self, id: u64, entry: weld::SnapshotLogEntry) {
        let row_name = format!("{}/{}", SNAPSHOTS, id);
        let snapshot_id = self.db.reserve_id(SNAPSHOT_IDS, &id.to_string());
        self.db
            .write_proto(&row_name, &snapshot_id.to_string(), 0, &entry);
    }

    pub fn get_change(&self, id: u64) -> Option<weld::Change> {
        self.db
            .read_proto(CHANGE_METADATA, &change_to_rowname(id), 0)
    }

    pub fn update_change(&self, change: &weld::Change) {
        self.spaces
            .write()
            .unwrap()
            .insert(change.get_friendly_name().to_owned(), change.get_id());

        self.db.write_proto(
            CHANGE_METADATA,
            &change_to_rowname(change.get_id()),
            0,
            change,
        );
    }

    pub fn list_changes(&self) -> impl Iterator<Item = weld::Change> + '_ {
        largetable_client::LargeTableScopedIterator::new(
            &self.db,
            String::from(CHANGE_METADATA),
            String::from(""),
            String::from(""),
            String::from(""),
            0,
        )
        .map(|(_, change)| change)
    }

    pub fn list_changed_files(&self, id: u64, index: u64) -> impl Iterator<Item = File> + '_ {
        largetable_client::LargeTableScopedIterator::new(
            &self.db,
            change_to_rowname(id),
            String::from(""),
            String::from(""),
            String::from(""),
            index,
        )
        .map(|(_, f)| f)
    }

    pub fn list_snapshots(&self, id: u64) -> impl Iterator<Item = weld::SnapshotLogEntry> + '_ {
        largetable_client::LargeTableScopedIterator::new(
            &self.db,
            format!("{}/{}", SNAPSHOTS, id),
            String::from(""),
            String::from(""),
            String::from(""),
            0,
        )
        .map(|(_, f)| f)
    }

    pub fn list_files_remote(&self, id: u64, directory: &str, index: u64) -> Vec<File> {
        match self.remote_server {
            Some(ref client) => {
                let mut ident = weld::FileIdentifier::new();
                ident.set_id(id);
                ident.set_filename(directory.to_owned());
                ident.set_index(index);
                client.list_files(ident)
            }
            None => vec![],
        }
    }

    pub fn list_files(&self, id: u64, directory: &str, index: u64) -> Vec<File> {
        // Need to make sure the last char in the string is a slash. Append one
        // if neccessary.
        let directory = normalize_directory(directory);

        let change = match self.get_change(id) {
            Some(c) => c,
            None => return vec![],
        };

        let mut files = std::collections::BTreeMap::new();
        for (_, file) in largetable_client::LargeTableScopedIterator::<File, _>::new(
            &self.db,
            change_to_rowname(id),
            path_to_colname(&directory),
            String::from(""),
            String::from(""),
            index,
        ) {
            files.insert(file.get_filename().to_owned(), file);
        }

        let based_files = if change.get_is_based_locally() {
            self.list_files(change.get_based_id(), &directory, change.get_based_index())
        } else {
            self.list_files_remote(change.get_based_id(), &directory, change.get_based_index())
        };

        for file in based_files {
            // Only insert if we don't already have a file for that filename.
            files.entry(file.get_filename().to_owned()).or_insert(file);
        }

        files
            .into_iter()
            .map(|(_, f)| f)
            .filter(|f| !f.get_deleted())
            .collect()
    }

    pub fn reserve_change_id(&self) -> u64 {
        self.db.reserve_id(CHANGE_IDS, "")
    }

    pub fn populate_change(&self, mut change: weld::Change) -> weld::Change {
        change.set_found(true);

        // First, get a list of all files touched by this change.
        // Then, go through all the snapshots. If there's a rebase, then insert
        // the version of the file at that moment.
        let snapshot_history = self.list_snapshots(change.get_id()).collect::<Vec<_>>();
        let mut files = HashMap::new();

        for snapshot in snapshot_history.iter() {
            for file in self.list_changed_files(change.get_id(), snapshot.get_index()) {
                let mut h = weld::FileHistory::new();
                h.set_filename(file.get_filename().to_owned());
                files.insert(file.get_filename().to_owned(), h);
            }
        }

        for (snapshot_id, snapshot) in snapshot_history.iter().enumerate() {
            // If the snapshot entry is a rebase, we need to pull all changed files
            // and enter the original version at this rebase.
            if snapshot.get_is_rebase() {
                for (_, history) in files.iter_mut() {
                    let mut ident = weld::FileIdentifier::new();
                    ident.set_id(snapshot.get_based_id());
                    ident.set_filename(history.get_filename().to_owned());
                    ident.set_index(snapshot.get_based_index());

                    let maybe_based_file = match change.get_is_based_locally() {
                        true => self.read(
                            snapshot.get_based_id(),
                            history.get_filename(),
                            snapshot.get_based_index(),
                        ),
                        false => self.read_remote(
                            snapshot.get_based_id(),
                            history.get_filename(),
                            snapshot.get_based_index(),
                        ),
                    };

                    if let Some(mut based_file) = maybe_based_file {
                        based_file.set_snapshot_id(snapshot_id as u64);
                        based_file.set_change_id(snapshot.get_based_index());
                        history.mut_snapshots().push(based_file);
                    }
                }

                continue;
            }
            // If it's not a rebase, that means we just need to include the changed
            // files in here.
            for mut file in self.list_changed_files(change.get_id(), snapshot.get_based_index()) {
                file.set_snapshot_id(snapshot_id as u64);
                file.set_change_id(0);
                let history = files.get_mut(file.get_filename()).unwrap();
                history.mut_snapshots().push(file);
            }
        }

        change.mut_changes().clear();
        for (_, history) in files.into_iter() {
            change.mut_changes().push(history);
        }

        change
    }

    pub fn snashot_from_id(&self, id: u64) -> weld::SnapshotResponse {
        let mut c = weld::Change::new();
        c.set_id(id);
        self.snapshot(&c)
    }

    pub fn snapshot(&self, partial_change: &weld::Change) -> weld::SnapshotResponse {
        let mut change = match self.get_change(partial_change.get_id()) {
            Some(c) => c,
            None => return weld::SnapshotResponse::new(),
        };

        // Use the fields from the partial change to update the change.
        weld::deserialize_change(&weld::serialize_change(partial_change, false), &mut change)
            .unwrap();

        self.update_change(&change);

        // Create an entry in the SNAPSHOTS record with the current filesystem state.
        let id = partial_change.get_id();
        let mut entry = weld::SnapshotLogEntry::new();
        let snapshot_id = weld::get_timestamp_usec();
        entry.set_index(snapshot_id);
        self.log_snapshot(id, entry);

        for file in self.list_changed_files(id, 0) {
            // Look up the remote file to figure out whether this file is identical to
            // the based version.
            let maybe_based_file = match change.get_is_based_locally() {
                true => self.read(
                    change.get_based_id(),
                    file.get_filename(),
                    change.get_based_index(),
                ),
                false => self.read_remote(
                    change.get_based_id(),
                    file.get_filename(),
                    change.get_based_index(),
                ),
            };

            let based_file = match maybe_based_file {
                Some(f) => f,
                None => weld::File::new(),
            };

            // If this file is a deletion, and he same file didn't exist in the remote repo,
            // then this is a no-op, and skip the file.
            if file.get_deleted() && !based_file.get_found() {
                continue;
            }

            // If the two protos are identical, then there's no change here, so ignore it.
            if file == based_file {
                continue;
            }

            change.mut_staged_files().push(file);
        }

        // If we are basing against a remote souce, report the snapshot back to the remote source.
        if change.get_is_based_locally() || self.remote_server.is_none() {
            let mut response = weld::SnapshotResponse::new();
            response.set_change_id(change.get_id());
            response.set_snapshot_id(snapshot_id);

            return response;
        }

        // Since this is going to the remote server, we need to reframe the change into the remote
        // server's frame. That means converting the is_based_locally to true and setting the
        // remote_id to the real id.
        let mut remote_change = change.clone();
        remote_change.set_id(change.get_remote_id());
        remote_change.set_is_based_locally(true);

        let response = self.remote_server.as_ref().unwrap().snapshot(remote_change);

        // Potentially update the pending ID, if one was assigned.
        if change.get_remote_id() != response.get_change_id() {
            // Strip out the staged files since they might be a lot of data.
            change.mut_staged_files().clear();
            change.set_remote_id(response.get_change_id());
        }
        self.update_change(&change);

        response
    }

    pub fn submit(&self, id: u64) -> weld::SubmitResponse {
        let change = match self.get_change(id) {
            Some(c) => c,
            None => return weld::SubmitResponse::new(),
        };
        self.remote_server.as_ref().unwrap().submit(change)
    }
}

fn parent_directory(filename: &str) -> String {
    // Remove the trailing slash, if it exists.
    let trimmed_filename = filename.trim_matches('/');

    let filename_parts: Vec<&str> = trimmed_filename.split('/').collect();
    let mut directory = String::from("/");
    for index in 0..filename_parts.len() - 1 {
        directory += filename_parts[index];
        if index != filename_parts.len() - 2 {
            directory += "/";
        }
    }

    directory
}

pub fn normalize_directory(directory: &str) -> String {
    format!("{}/", normalize_filename(directory).trim_right_matches('/'))
}

pub fn normalize_filename(filename: &str) -> String {
    format!("/{}", filename.trim_matches('/'))
}

fn change_to_rowname(id: u64) -> String {
    format!("{}/{}", CHANGES, id)
}

pub fn path_to_colname(path: &str) -> String {
    let depth = path.split("/").count();
    format!("{}:{}", depth, path)
}

#[cfg(test)]
mod tests {
    extern crate largetable_test;
    extern crate weld_test;
    use super::*;
    use std::sync::Arc;

    type TestRepo = Repo<largetable_test::LargeTableMockClient, weld_test::WeldServerTestClient>;

    #[test]
    fn test_parent_directory() {
        assert_eq!(parent_directory("/a/b/c/d/"), String::from("/a/b/c"));
        assert_eq!(parent_directory("/a/b/c/d/"), String::from("/a/b/c"));

        assert_eq!(parent_directory("/test/file.txt"), String::from("/test"));
        assert_eq!(parent_directory("/a"), String::from("/"));
        assert_eq!(
            parent_directory(&parent_directory("/a/b")),
            String::from("/")
        );
    }

    fn make_test_repo() -> TestRepo {
        Repo::new(largetable_test::LargeTableMockClient::new())
    }

    fn make_remote_connected_test_repo() -> TestRepo {
        let mut repo = make_test_repo();
        let remote = weld_test::WeldServerTestClient::new(String::from("tester"));
        repo.add_remote_server(remote);
        repo
    }

    #[test]
    fn test_make_get_change() {
        let repo = make_test_repo();
        let mut change = weld::Change::new();
        change.set_friendly_name(String::from("test"));
        let id = repo.make_change(change);

        let response = repo.get_change(id).unwrap();
        assert_eq!(id, response.get_id());
        assert_eq!(String::from("test"), response.get_friendly_name());
        assert_eq!(true, response.get_found());
    }

    #[test]
    fn test_read_write_file() {
        let repo = make_test_repo();
        let id = repo.make_change(weld::Change::new());

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("/test/config.txt").into_bytes());

        repo.write(id, test_file.clone(), 0);

        test_file.set_found(true);
        assert_eq!(repo.read(id, "/test/config.txt", 0), Some(test_file));
    }

    #[test]
    fn test_auto_create_dir() {
        let repo = make_test_repo();
        let id = repo.make_change(weld::Change::new());

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("/test/config.txt").into_bytes());

        repo.write(id, test_file, 0);

        let mut expected_dir = File::new();
        expected_dir.set_filename(String::from("/test"));
        expected_dir.set_directory(true);
        expected_dir.set_found(true);

        assert_eq!(repo.read(id, "/test", 0), Some(expected_dir.clone()));
        assert_eq!(repo.read(id, "/test/", 0), Some(expected_dir));
    }

    #[test]
    fn test_list_spaces() {
        let repo = make_test_repo();
        let mut c = weld::Change::new();
        c.set_friendly_name(String::from("test"));
        repo.make_change(c);

        let mut c = weld::Change::new();
        c.set_friendly_name(String::from("another_one"));
        repo.make_change(c);

        assert_eq!(
            repo.list_changes()
                .map(|s| s.get_friendly_name().to_owned())
                .collect::<Vec<_>>(),
            vec!["test", "another_one"]
        );
    }

    #[test]
    fn test_list_files() {
        let repo = make_test_repo();
        let id = repo.make_change(weld::Change::new());

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(id, test_file, 0);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/test.cc"));
        test_file.set_contents(String::from("int main() { return -1; }").into_bytes());
        repo.write(id, test_file, 0);

        assert_eq!(
            repo.list_files(id, "/test", 0)
                .iter()
                .map(|x| x.get_filename())
                .collect::<Vec<_>>(),
            vec!["/test/config.txt", "/test/test.cc"]
        );
    }

    #[test]
    fn test_list_files_based_space() {
        let repo = make_test_repo();
        let based_id = repo.make_change(weld::Change::new());

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(based_id, test_file, 0);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/test.cc"));
        test_file.set_contents(String::from("int main() { return -1; }").into_bytes());
        repo.write(based_id, test_file, 0);

        let mut change = weld::Change::new();
        change.set_is_based_locally(true);
        change.set_based_id(based_id);

        let new_id = repo.make_change(change);

        assert_ne!(new_id, based_id, "The two IDs should be different");

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/change.txt"));
        test_file.set_contents(String::from("I added a file").into_bytes());
        repo.write(new_id, test_file, 0);

        assert_eq!(
            repo.list_files(new_id, "/test", 0)
                .iter()
                .map(|x| x.get_filename())
                .collect::<Vec<_>>(),
            vec!["/test/change.txt", "/test/config.txt", "/test/test.cc"]
        );
    }

    #[test]
    fn test_read_write() {
        let repo = make_test_repo();
        let id = repo.make_change(weld::Change::new());

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(id, test_file, 1);

        assert_eq!(
            repo.read(id, "/test/config.txt", 1).unwrap().get_found(),
            true
        );
    }

    #[test]
    fn list_changed_files() {
        let repo = make_test_repo();
        let id = repo.make_change(weld::Change::new());

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(id, test_file, 0);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/test.cc"));
        test_file.set_contents(String::from("int main() { return -1; }").into_bytes());
        repo.write(id, test_file, 0);

        assert_eq!(
            repo.list_changed_files(id, 0)
                .map(|x| x.get_filename().to_owned())
                .collect::<Vec<_>>(),
            vec!["/test", "/test/config.txt", "/test/test.cc"]
        );
    }

    #[test]
    fn check_change_creation() {
        let repo = make_test_repo();

        // First, set up the change we are basing on.
        let mut change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(id, test_file, 0);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/test.cc"));
        test_file.set_contents(String::from("int main() { return -1; }").into_bytes());
        repo.write(id, test_file, 0);
        let index = repo.snapshot(&weld::change(id)).get_snapshot_id();

        let change = repo.populate_change(repo.get_change(id).unwrap());
        assert_eq!(change.get_changes().len(), 3);

        // Now create a change based on that change.
        let mut change = weld::Change::new();
        change.set_based_id(id);
        change.set_based_index(index);
        change.set_is_based_locally(true);
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: false}").into_bytes());
        repo.write(id, test_file, 0);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/test.h"));
        test_file.set_contents(String::from("int main(int argc, char* argv[]);").into_bytes());
        repo.write(id, test_file, 0);
        repo.snapshot(&weld::change(id));

        let change = repo.populate_change(repo.get_change(id).unwrap());
        assert_eq!(change.get_changes().len(), 2);
        assert_eq!(change.get_staged_files().len(), 0);

        // The order could be random, so let's store the output in a hash map.
        let mut map = HashMap::new();
        for c in change.get_changes() {
            map.insert(String::from(c.get_filename()), c.clone());
        }

        // Check that the expected number of changes are there.
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("/test/test.h").unwrap().get_snapshots().len(), 1);
        assert_eq!(
            map.get("/test/config.txt").unwrap().get_snapshots().len(),
            2
        );

        // Check that the revision from the based repo is in there.
        assert_eq!(
            std::str::from_utf8(
                map.get("/test/config.txt")
                    .unwrap()
                    .get_snapshots()
                    .get(0)
                    .unwrap()
                    .get_contents()
            )
            .unwrap(),
            "{config: true}"
        );
    }

    #[test]
    fn test_remote_server_interaction() {
        let repo = make_remote_connected_test_repo();

        // Make a change and submit it
        let mut change = weld::Change::new();
        let old_id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(old_id, test_file, 0);

        let index = repo.snapshot(&weld::change(old_id)).get_snapshot_id();
        let submitted_id = repo.submit(old_id).get_id();

        // Snapshot takes id 1, then submitted id is 2
        assert_eq!(submitted_id, 2);

        // Make another change and submit it
        let mut change = weld::Change::new();
        let new_id = repo.make_change(change);

        // Inspect returned change.
        let change = repo.get_change(new_id).unwrap();
        assert_eq!(change.get_is_based_locally(), false);
        assert_eq!(change.get_based_id(), 0);
        assert_eq!(change.get_based_index(), 2);

        // Try directly reading remote repo.
        assert!(
            repo.read_remote(0, "/test/config.txt", 2).is_some(),
            "Unable to read submitted file"
        );

        assert_ne!(new_id, old_id, "New change should be a new index");

        // Under this new change, we expect to inherit the old change.
        let maybe_file = repo.read(new_id, "/test/config.txt", 0);

        assert!(maybe_file.is_some(), "Should get file, but didn't");

        assert_eq!(
            std::str::from_utf8(maybe_file.unwrap().get_contents()).unwrap(),
            "{config: true}"
        );

        // Not only try reading the file, but also listing the directory.
        let listing = repo.list_files(2, "/test", 0);
        assert_eq!(listing.len(), 1, "Should list one file");
        assert_eq!(listing[0].get_filename(), "/test/config.txt");

        // Also try listing remotely.
        let listing = repo.list_files_remote(0, "/test", 2);
        assert_eq!(listing.len(), 1, "Should list one file");
        assert_eq!(listing[0].get_filename(), "/test/config.txt");

        // Modify the file and take snapshot
        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: false}").into_bytes());
        repo.write(new_id, test_file, 0);

        // Provide a description to the snapshot
        let mut c = weld::change(new_id);
        c.set_description(String::from("Hello, world"));
        let pending_id = repo.snapshot(&c).get_change_id();

        // Check whether the produced change is sensible
        let change = (repo.remote_server.as_ref().unwrap() as &weld::WeldServer)
            .get_change(weld::change(pending_id));

        println!("change: {}", text_format::print_to_string(&change));

        assert!(change.get_found(), "Couldn't find change for snapshot");
        assert_eq!(change.get_description(), "Hello, world");
        assert_eq!(
            change.get_staged_files().len(),
            0,
            "Shouldn't have staged changes in response"
        );
        assert_eq!(
            change.get_changes().len(),
            1,
            "Should have one changed file"
        );
        assert_eq!(
            change.get_changes().get(0).unwrap().get_filename(),
            "/test/config.txt",
            "Should have one changed file"
        );
        assert_eq!(
            change.get_changes().get(0).unwrap().get_snapshots().len(),
            2,
            "Should have two snapshots: original based snapshot and modification"
        );
        assert_eq!(
            change
                .get_changes()
                .get(0)
                .unwrap()
                .get_snapshots()
                .get(0)
                .unwrap()
                .get_change_id(),
            2,
            "Original change should be based on change ID 2"
        );
        assert_eq!(
            change
                .get_changes()
                .get(0)
                .unwrap()
                .get_snapshots()
                .get(1)
                .unwrap()
                .get_change_id(),
            0,
            "New change should have change ID zero to indicate it's modified"
        );
    }

    #[test]
    fn test_multiple_snapshots() {
        let repo = make_remote_connected_test_repo();

        // Make a change with a modified file
        let mut change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(id, test_file, 0);

        // Take snapshot
        let index = repo.snapshot(&weld::change(id)).get_snapshot_id();

        // Now change that file again.
        let mut test_file = File::new();
        test_file.set_filename(String::from("/config.txt"));
        test_file.set_contents(String::from("{config: false}").into_bytes());
        repo.write(id, test_file, 0);

        // Now do another snapshot.
        repo.snapshot(&weld::change(id));

        // Check whether the produced change is sensible
        let change = (repo.remote_server.as_ref().unwrap() as &weld::WeldServer)
            .get_change(weld::change(id));

        println!("change: {}", text_format::print_to_string(&change));

        assert!(change.get_found(), "Couldn't find change for snapshot");
        assert_eq!(
            change.get_staged_files().len(),
            0,
            "Shouldn't have staged changes in response"
        );
        assert_eq!(
            change.get_changes().len(),
            1,
            "Should have one changed file"
        );
        assert_eq!(
            change.get_changes().get(0).unwrap().get_filename(),
            "/config.txt",
            "Should have one changed file"
        );
        assert_eq!(
            change.get_changes().get(0).unwrap().get_snapshots().len(),
            2,
            "Should have two snapshots."
        );
    }
}
