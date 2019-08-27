#[cfg(test)]
mod tests {
    extern crate largetable_test;
    extern crate weld;
    extern crate weld_repo;
    extern crate weld_test;

    use self::weld::File;
    use self::weld::WeldServer;
    use self::weld_repo::*;

    use std::collections::HashMap;

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

        let mut test_dir = File::new();
        test_dir.set_filename(String::from("/test_directory"));
        test_dir.set_directory(true);
        repo.write(id, test_dir, 0);

        let mut expected_dir = File::new();
        expected_dir.set_filename(String::from("/test"));
        expected_dir.set_directory(true);
        expected_dir.set_found(true);

        assert_eq!(repo.read(id, "/test", 0), Some(expected_dir.clone()));
        assert_eq!(repo.read(id, "/test/", 0), Some(expected_dir));

        assert_eq!(
            repo.list_files(id, "/", 0)
                .iter()
                .map(|x| x.get_filename())
                .collect::<Vec<_>>(),
            vec!["/test", "/test_directory"]
        );
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
        let change = weld::Change::new();
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
        let change = weld::Change::new();
        let old_id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(old_id, test_file, 0);

        repo.snapshot(&weld::change(old_id)).get_snapshot_id();
        let submitted_id = repo.submit(old_id).get_id();

        // Snapshot takes id 2, then submitted id is 3
        assert_eq!(submitted_id, 3);

        // Make another change and submit it
        let change = weld::Change::new();
        let new_id = repo.make_change(change);

        // Inspect returned change.
        let change = repo.get_change(new_id).unwrap();
        assert_eq!(change.get_is_based_locally(), false);
        assert_eq!(change.get_based_id(), 0);
        assert_eq!(change.get_based_index(), 3);

        // Try directly reading remote repo.
        assert!(
            repo.read_remote(0, "/test/config.txt", 3).is_some(),
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
        let listing = repo.list_files_remote(0, "/test", 3);
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
            3,
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
        let change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(id, test_file, 0);

        // Take snapshot
        repo.snapshot(&weld::change(id)).get_snapshot_id();

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

    #[test]
    fn test_cache() {
        let repo = make_remote_connected_test_repo();

        // Make a change with a modified file
        let change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(id, test_file, 0);

        // Take snapshot
        let index = repo.snapshot(&weld::change(id)).get_snapshot_id();

        let response = repo.read_remote(id, "/config.txt", index);
        assert!(response.is_some());

        let response = repo.read_remote(id, "/config.txt", index);
        assert!(response.is_some());
    }

    #[test]
    fn test_reboot() {
        let db = largetable_test::LargeTableMockClient::new();
        {
            let repo: TestRepo = Repo::new(db.clone());
            let mut c = weld::Change::new();
            c.set_friendly_name(String::from("test"));
            repo.make_change(c);
            assert_eq!(repo.list_changes().count(), 1);
            assert!(repo.lookup_friendly_name("test").is_some());
        }

        {
            let repo: TestRepo = Repo::new(db);
            assert_eq!(repo.list_changes().count(), 1);
            assert!(repo.lookup_friendly_name("test").is_some());
        }
    }

    #[test]
    fn test_list_changes() {
        let repo = make_remote_connected_test_repo();

        // Make a change and submit it
        let change = weld::Change::new();
        let old_id = repo.make_change(change);

        // The change should show up when listing changes.
        assert_eq!(repo.list_changes().count(), 1);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/test/config.txt"));
        test_file.set_contents(String::from("{config: true}").into_bytes());
        repo.write(old_id, test_file, 0);

        repo.snapshot(&weld::change(old_id)).get_snapshot_id();
        repo.submit(old_id).get_id();

        // After the change is submitted, it shouldn't appear in list_changes
        assert_eq!(repo.list_changes().count(), 0);
    }

    #[test]
    fn test_multiple_submits() {
        let repo = make_remote_connected_test_repo();

        println!("[test] Write a README file");
        {
            let change = weld::Change::new();
            let id = repo.make_change(change);

            let mut test_file = File::new();
            test_file.set_filename(String::from("/README.md"));
            test_file.set_contents(String::from("test content").into_bytes());
            repo.write(id, test_file, 0);

            repo.snapshot(&weld::change(id)).get_change_id();
            let submitted_id = repo.submit(id).get_id();

            assert_ne!(submitted_id, 0, "Error submitting change");
        }

        println!("[test] Make a change on top of the README.md file");
        {
            let change = weld::Change::new();
            let id = repo.make_change(change);

            let f = repo.read(id, "/README.md", 0).unwrap();
            assert_eq!(f.get_contents(), String::from("test content").as_bytes());

            let mut test_file = weld::File::new();
            test_file.set_filename(String::from("/README.md"));
            test_file.set_contents(String::from("test content\nextra content").into_bytes());
            repo.write(id, test_file, 0);

            repo.snapshot(&weld::change(id)).get_change_id();
            let submitted_id = repo.submit(id).get_id();

            assert_ne!(submitted_id, 0, "Error submitting change");
        }
    }

    #[test]
    fn test_several_updates() {
        let repo = make_remote_connected_test_repo();

        let change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/README.md"));
        test_file.set_contents(String::from("test content").into_bytes());
        repo.write(id, test_file, 0);

        repo.snapshot(&weld::change(id));

        let changes = repo.remote_server.as_ref().unwrap().list_changes();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].get_changes().len(), 1);
        assert_eq!(changes[0].get_changes()[0].get_filename(), "/README.md");

        let mut test_file = File::new();
        test_file.set_filename(String::from("/README2.md"));
        test_file.set_contents(String::from("more test content").into_bytes());
        repo.write(id, test_file, 0);

        repo.snapshot(&weld::change(id));

        let changes = repo.remote_server.unwrap().list_changes();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].get_changes().len(), 2);
    }

    #[test]
    fn test_conflict() {
        let repo = make_remote_connected_test_repo();

        {
            // Dumb hack - need to submit an initial change in order to not end up with a based
            // index of 0 which is the canary for HEAD
            let change = weld::Change::new();
            let id = repo.make_change(change);

            let mut test_file = File::new();
            test_file.set_filename(String::from("/README.md"));
            test_file.set_contents(String::from("initial content").into_bytes());
            repo.write(id, test_file, 0);

            repo.snapshot(&weld::change(id)).get_change_id();
            let submitted_id = repo.submit(id).get_id();

            assert_ne!(submitted_id, 0, "Error submitting change");
        }

        let change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/README.md"));
        test_file.set_contents(String::from("test content").into_bytes());
        repo.write(id, test_file, 0);

        repo.snapshot(&weld::change(id)).get_change_id();

        // Before submitting the first one, make a change based on previous state
        let change_b = weld::Change::new();
        let id_b = repo.make_change(change_b);

        let change = repo.get_change(id_b).unwrap();
        println!("change_b: {:?}", change);

        // Submit original change
        repo.submit(id).get_id();

        // Make a conflicting change
        let mut test_file = File::new();
        test_file.set_filename(String::from("/README.md"));
        test_file.set_contents(String::from("west content").into_bytes());
        repo.write(id_b, test_file, 0);

        repo.snapshot(&weld::change(id_b)).get_change_id();

        // Try to sync
        let (conflicting_files, _) = repo.sync(id_b, &[]);

        // Sync should fail due to conflicting files
        assert_eq!(conflicting_files.len(), 1);
    }

    #[test]
    fn test_mergeable_conflict() {
        let repo = make_remote_connected_test_repo();

        {
            // Dumb hack - need to submit an initial change in order to not end up with a based
            // index of 0 which is the canary for HEAD
            let change = weld::Change::new();
            let id = repo.make_change(change);

            let mut test_file = File::new();
            test_file.set_filename(String::from("/README.md"));
            test_file.set_contents(String::from("L1\nL2\nL3").into_bytes());
            repo.write(id, test_file, 0);

            repo.snapshot(&weld::change(id)).get_change_id();
            let submitted_id = repo.submit(id).get_id();

            assert_ne!(submitted_id, 0, "Error submitting change");
        }

        let change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/README.md"));
        test_file.set_contents(String::from("M1\nL2\nL3").into_bytes());
        repo.write(id, test_file, 0);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/signal.txt"));
        test_file.set_contents(String::from("hello world").into_bytes());
        repo.write(id, test_file, 0);

        repo.snapshot(&weld::change(id)).get_change_id();

        // Before submitting the first one, make a change based on previous state
        let change_b = weld::Change::new();
        let id_b = repo.make_change(change_b);

        let change = repo.get_change(id_b).unwrap();
        println!("change_b: {:?}", change);

        // Submit original change
        repo.submit(id).get_id();

        // Make a conflicting change
        let mut test_file = File::new();
        test_file.set_filename(String::from("/README.md"));
        test_file.set_contents(String::from("L1\nL2\nM3").into_bytes());
        repo.write(id_b, test_file, 0);

        repo.snapshot(&weld::change(id_b)).get_change_id();

        // Should be based on #3
        let change = repo.get_change(id_b).unwrap();
        assert_eq!(change.get_based_index(), 3);

        // Try to sync
        let (conflicting_files, _) = repo.sync(id_b, &[]);

        // Sync should succeed because merge is OK
        assert_eq!(conflicting_files.len(), 0);

        // Repo should have ok merge status
        assert_eq!(
            std::str::from_utf8(repo.read(id_b, "/README.md", 0).unwrap().get_contents()).unwrap(),
            "M1\nL2\nM3\n"
        );

        // Should now be based on #4
        let change = repo.get_change(id_b).unwrap();
        assert_eq!(change.get_based_index(), 5);

        // Now the other file should show up
        assert!(repo.read(id_b, "/signal.txt", 0).is_some());
    }

    #[test]
    fn test_huge_number_of_files() {
        let repo = make_remote_connected_test_repo();

        // index of 0 which is the canary for HEAD
        let change = weld::Change::new();
        let id = repo.make_change(change);

        for index in 0..4096 {
            let mut test_file = File::new();
            test_file.set_filename(format!("/file{}.txt", index));
            test_file.set_contents(String::from("initial content").into_bytes());
            repo.write(id, test_file, 0);
        }

        assert_eq!(repo.list_changed_files(id, 0).count(), 4096);

        repo.snapshot(&weld::change(id)).get_change_id();
        let submitted_id = repo.submit(id).get_id();

        assert_ne!(submitted_id, 0);
    }

    #[test]
    fn test_manually_merged_conflict() {
        let repo = make_remote_connected_test_repo();

        {
            // Dumb hack - need to submit an initial change in order to not end up with a based
            // index of 0 which is the canary for HEAD
            let change = weld::Change::new();
            let id = repo.make_change(change);

            let mut test_file = File::new();
            test_file.set_filename(String::from("/README.md"));
            test_file.set_contents(String::from("initial content").into_bytes());
            repo.write(id, test_file, 0);

            repo.snapshot(&weld::change(id)).get_change_id();
            let submitted_id = repo.submit(id).get_id();

            assert_ne!(submitted_id, 0, "Error submitting change");
        }

        let change = weld::Change::new();
        let id = repo.make_change(change);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/README.md"));
        test_file.set_contents(String::from("conflict1").into_bytes());
        repo.write(id, test_file, 0);

        let mut test_file = File::new();
        test_file.set_filename(String::from("/signal.txt"));
        test_file.set_contents(String::from("hello world").into_bytes());
        repo.write(id, test_file, 0);

        repo.snapshot(&weld::change(id)).get_change_id();

        // Before submitting the first one, make a change based on previous state
        let change_b = weld::Change::new();
        let id_b = repo.make_change(change_b);

        let change = repo.get_change(id_b).unwrap();
        println!("change_b: {:?}", change);

        // Submit original change
        repo.submit(id).get_id();

        // Make a conflicting change
        let mut test_file = File::new();
        test_file.set_filename(String::from("/README.md"));
        test_file.set_contents(String::from("conflict2").into_bytes());
        repo.write(id_b, test_file, 0);

        repo.snapshot(&weld::change(id_b)).get_change_id();

        // Should be based on #2
        let change = repo.get_change(id_b).unwrap();
        assert_eq!(change.get_based_index(), 3);

        // Try to sync with manual merge
        let mut manual_merge = File::new();
        manual_merge.set_filename(String::from("/README.md"));
        manual_merge.set_contents(String::from("conflict1, conflict2").into_bytes());
        let (conflicting_files, _) = repo.sync(id_b, &[manual_merge]);

        // Sync should succeed because merge is OK
        assert_eq!(conflicting_files.len(), 0);

        // Repo should have ok merge status
        assert_eq!(
            std::str::from_utf8(repo.read(id_b, "/README.md", 0).unwrap().get_contents()).unwrap(),
            "conflict1, conflict2"
        );

        // Should now be based on #4
        let change = repo.get_change(id_b).unwrap();
        assert_eq!(change.get_based_index(), 5);

        // Now the other file should show up
        assert!(repo.read(id_b, "/signal.txt", 0).is_some());

        // Try to sync again (already up to date)
        let (conflicting_files, synced_to) = repo.sync(id_b, &[]);
        assert_eq!(conflicting_files.len(), 0);
        assert_eq!(synced_to, 5);
    }
}
