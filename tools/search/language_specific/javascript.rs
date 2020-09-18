use search_proto_rust::*;

lazy_static! {
    static ref CLASS_BINDING: regex::Regex =
        { regex::Regex::new(r"\s*(export\s)?\s*(default\s)?\s*(class)\s+(\w+)").unwrap() };
    static ref ANONYMOUS_FUNCTION: regex::Regex = {
        regex::Regex::new(r"^\s*(export\s)?\s*(default\s)?\s*(const|let|var|static)?\s*(\w+)\s*=\s*\([\w\s,=]*\)\s*[=-]>\s*\{").unwrap()
    };
    static ref FUNCTION_DEFINITION: regex::Regex =
        { regex::Regex::new(r"\s*(export\s)?\s*(default\s)?\s*(function)\s+(\w+)").unwrap() };
    static ref FUNCTION_DEFINITION_IN_CLASS: regex::Regex =
        { regex::Regex::new(r"^\s*(\w+)\([\w\s,=]*\)\s*\{").unwrap() };
    static ref VAR_BINDING: regex::Regex = {
        regex::Regex::new(r"\s*(export\s)?\s*(default\s)?\s*(const|let|var|static)\s+(\w+)")
            .unwrap()
    };
    static ref PROPERTY_DEFINITION: regex::Regex = { regex::Regex::new(r"(\w+)\s*:").unwrap() };
    static ref IMPORT_DEFINITION: regex::Regex =
        { regex::Regex::new(r"(?m:^)\s*import(?sUm:\s.*)from\s+'([^']+)'").unwrap() };
}

pub fn annotate_file(file: &mut File) {
    if file.get_filename().ends_with(".jest.js")
        || file.get_filename().ends_with("-test.js")
        || file.get_filename().ends_with("-jest.js")
        || file.get_filename().ends_with(".test.js")
        || file.get_filename().ends_with(".test.ts")
        || file.get_filename().ends_with(".snap")
        || file.get_filename().ends_with(".ambr")
        || file.get_filename().contains("/__tests__")
        || file.get_filename().contains("/__snapshots__")
        || file.get_filename().contains("_test/")
    {
        file.set_is_test(true);
    }
}

pub fn extract_definitions(file: &File) -> Vec<SymbolDefinition> {
    let mut results = Vec::new();
    'outer: for (line_number, line) in file.get_content().lines().enumerate() {
        for captures in PROPERTY_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::VARIABLE);
            results.push(d);
        }
        for captures in CLASS_BINDING.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::STRUCTURE);
            results.push(d);
            continue 'outer;
        }
        for captures in ANONYMOUS_FUNCTION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::FUNCTION);
            results.push(d);
            continue 'outer;
        }
        for captures in FUNCTION_DEFINITION.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::FUNCTION);
            results.push(d);
            continue 'outer;
        }
        for captures in FUNCTION_DEFINITION_IN_CLASS.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::FUNCTION);
            results.push(d);
            continue 'outer;
        }
        for captures in VAR_BINDING.captures_iter(line) {
            let mut d = SymbolDefinition::new();
            d.set_symbol(captures[captures.len() - 1].to_string());
            d.set_filename(file.get_filename().to_string());
            d.set_line_number(line_number as u32);
            d.set_symbol_type(SymbolType::VARIABLE);
            results.push(d);
        }
    }
    results
}

pub fn extract_imports(file: &File) -> Vec<String> {
    let mut results = Vec::new();
    for captures in IMPORT_DEFINITION.captures_iter(file.get_content()) {
        let import_path = &captures[captures.len() - 1];

        if !import_path.ends_with(".js")
            && !import_path.ends_with(".mjs")
            && !import_path.ends_with(".ts")
        {
            // If the ending is not specified, then we should check any valid
            // javascript file ending to see if any exist
            results.push(format!("{}.js", import_path));
            results.push(format!("{}.mjs", import_path));
            results.push(format!("{}.ts", import_path));
            results.push(format!("{}/index.js", import_path));
        } else {
            results.push(import_path.to_string());
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kw(word: &str, typ: SymbolType, line: u32) -> SymbolDefinition {
        let mut xk = SymbolDefinition::new();
        xk.set_symbol(word.to_owned());
        xk.set_symbol_type(typ);
        xk.set_line_number(line);
        xk
    }

    #[test]
    fn test_extract_imports() {
        let mut f = File::new();
        f.set_content(
            "
            import { abcdef }, xyz from './utils/docs/code.js';
            import xyz from 'fake/path/here.mjs';
            import { 
                abcdef,
                cdefg,
                qrst}, xyz 
            from './my/relative/path.ts';
            import qqq from 'test/path'
            "
            .into(),
        );

        let result = extract_imports(&f);
        assert_eq!(result[0], "./utils/docs/code.js");
        assert_eq!(result[1], "fake/path/here.mjs");
        assert_eq!(result[2], "./my/relative/path.ts");
        assert_eq!(result[3], "test/path.js");
        assert_eq!(result[4], "test/path.mjs");
        assert_eq!(result[5], "test/path.ts");
        assert_eq!(result[6], "test/path/index.js");
    }

    #[test]
    fn test_extract_definitions() {
        let mut f = File::new();
        f.set_content(
            "
    export default class EmojiResults extends PureComponent {
  static propTypes = {
    emojiData: PropTypes.array
  };

  componentDidMount() {
    this.clipboard = new Clipboard();
  }

  componentWillUnmount() {
    this.clipboard.destroy();
  }}

  export function exploder(x, y, z) {
      const garble = 5;
      const xyz = { gorble: true, sporgle: 9 };
  }
  "
            .into(),
        );

        let extracted = extract_definitions(&f);

        assert_eq!(&extracted[0], &kw("EmojiResults", SymbolType::STRUCTURE, 1));
        assert_eq!(&extracted[1], &kw("propTypes", SymbolType::VARIABLE, 2));
        assert_eq!(&extracted[2], &kw("emojiData", SymbolType::VARIABLE, 3));
        assert_eq!(
            &extracted[3],
            &kw("componentDidMount", SymbolType::FUNCTION, 6)
        );
        assert_eq!(
            &extracted[4],
            &kw("componentWillUnmount", SymbolType::FUNCTION, 10)
        );
        assert_eq!(&extracted[5], &kw("exploder", SymbolType::FUNCTION, 14));
        assert_eq!(&extracted[6], &kw("garble", SymbolType::VARIABLE, 15));
        assert_eq!(&extracted[7], &kw("gorble", SymbolType::VARIABLE, 16));
        assert_eq!(&extracted[8], &kw("sporgle", SymbolType::VARIABLE, 16));
        assert_eq!(&extracted[9], &kw("xyz", SymbolType::VARIABLE, 16));
    }
}
