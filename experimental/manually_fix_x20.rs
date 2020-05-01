fn main() {
    let f = std::fs::File::open("/tmp/configs.recordio").unwrap();
    let records = recordio::RecordIOReaderOwned::<x20_grpc_rust::Configuration>::new(Box::new(f));

    let f = std::fs::File::create("/tmp/config-fixed.recordio").unwrap();
    let mut writer = recordio::RecordIOWriter::new(f);
    for mut record in records {
        if record.get_name() == "x20" {
            let mut new_args = Vec::new();
            for arg in record.get_arguments() {
                if arg.get_name() != "auth_hostname" {
                    new_args.push(arg.clone());
                }
            }
            record.mut_arguments().clear();
            for arg in new_args {
                record.mut_arguments().push(arg);
            }
        }
        println!("{:?}\n", record);
        writer.write(&record);
    }

    let f = std::fs::File::open("/tmp/binaries.recordio").unwrap();
    let records = recordio::RecordIOReaderOwned::<x20_grpc_rust::Binary>::new(Box::new(f));

    let f = std::fs::File::create("/tmp/binaries-fixed.recordio").unwrap();
    let mut writer = recordio::RecordIOWriter::new(f);
    for mut binary in records {
        if binary.get_name() == "x20_server" {
            binary.set_docker_img_tag(String::from(
                "sha256:5b9ed7175ba86ed9155e687e17cf907345e8665f4e8703f24b4d0c2e2ebf0c67",
            ));
        }
        println!("{:?}\n", binary);
        writer.write(&binary);
    }
}
