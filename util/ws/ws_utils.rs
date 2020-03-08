#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;

lazy_static! {
    static ref RESERVED_CHARS: HashMap<char, &'static str> = {
        let mut map = HashMap::new();
        map.insert('!', "%21");
        map.insert('#', "%23");
        map.insert('$', "%24");
        map.insert('&', "%25");
        map.insert('\'', "%27");
        map.insert('(', "%28");
        map.insert(')', "%29");
        map.insert('*', "%2A");
        map.insert('+', "%2B");
        map.insert(',', "%2C");
        map.insert('/', "%2F");
        map.insert(':', "%3A");
        map.insert(';', "%3B");
        map.insert('=', "%3D");
        map.insert('?', "%3F");
        map.insert('@', "%40");
        map.insert('[', "%5B");
        map.insert(']', "%5D");
        map.insert(' ', "%20");
        map.insert('\n', "%0A");
        map.insert('\"', "%22");
        map.insert('%', "%25");
        map.insert('-', "%2D");
        map.insert('<', "%3C");
        map.insert('>', "%3E");
        map
    };
    static ref REVERSE_RESERVED_CHARS: HashMap<&'static str, char> = {
        let mut map = HashMap::new();
        for (k, v) in RESERVED_CHARS.iter() {
            map.insert(v.clone(), k.to_owned());
        }
        map
    };
}

pub fn urlencode(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        if let Some(code) = RESERVED_CHARS.get(&ch) {
            output += code
        } else {
            output.push(ch)
        }
    }
    output
}

pub fn urldecode(input: &str) -> String {
    let mut output = String::new();
    let mut chiter = input.chars();
    loop {
        let c = match chiter.next() {
            Some(c) => c,
            None => break,
        };

        if c == '%' {
            let mut code = String::new();
            code.push('%');

            let c = match chiter.next() {
                Some(c) => c,
                None => {
                    output += &code;
                    break;
                }
            };
            code.push(c);

            let c = match chiter.next() {
                Some(c) => c,
                None => {
                    output += &code;
                    break;
                }
            };
            code.push(c);

            if let Some(decode) = REVERSE_RESERVED_CHARS.get(code.as_str()) {
                output.push(*decode);
            } else {
                output += &code;
            }
        } else {
            output.push(c);
        }
    }
    output
}

pub fn parse_params(params: &str) -> HashMap<String, String> {
    let mut output = HashMap::new();
    for param in params.split("&") {
        if let Some(idx) = param.find("=") {
            let (key, value) = param.split_at(idx);
            if value.len() > 0 {
                output.insert(key.to_owned(), urldecode(&value[1..]));
            } else {
                output.insert(key.to_owned(), String::from(""));
            }
        } else {
            output.insert(param.to_owned(), String::from(""));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(
            urlencode("http://hello_world.com"),
            "http%3A%2F%2Fhello_world.com"
        );
    }

    #[test]
    fn test_parse() {
        let p = parse_params("parameter1=true&parameter2=false&parm3=a%20space%20here");
        assert_eq!(p.get("parameter1").unwrap(), "true");
        assert_eq!(p.get("parm3").unwrap(), "a space here");
    }

    #[test]
    fn test_decode() {
        assert_eq!(urldecode("%2F%20asdf%20%2F"), "/ asdf /");
        assert_eq!(urldecode("my%20dog%20has%20fleas"), "my dog has fleas");
        assert_eq!(urldecode("my%20dog%20has%2"), "my dog has%2");
    }
}
