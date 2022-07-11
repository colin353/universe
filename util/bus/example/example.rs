use bus::Serialize;
use std::io::Read;

fn main() {
    let read = flags::define_flag!("read", false, "Whether or not to read from stdin");
    flags::parse_flags!(read);

    if read.value() {
        let mut buf = Vec::new();
        let mut stdin = std::io::stdin();
        stdin.read_to_end(&mut buf).unwrap();
        match schema::Data::from_bytes(&mut buf) {
            Ok(d) => println!("{:#?}", d),
            Err(_) => println!("error"),
        };
    } else {
        let mut data = schema::Data::new();
        data.id = 1234;
        data.name = String::from("asdf");
        data.payload = vec![0x0a, 0x0b, 0x0c];
        data.flag = false;

        let mut buf = Vec::new();
        data.encode(&mut buf).unwrap();

        let mut out = std::io::stdout();
        data.encode(&mut out).unwrap();
    }
}
