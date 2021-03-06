use plume::{EmitFn, PCollection, PTable, KV};
use search_proto_rust::*;

struct ExtractEntityInfoFromTargetsFn {}
impl plume::DoFn for ExtractEntityInfoFromTargetsFn {
    type Input = KV<String, Target>;
    type Output = KV<String, EntityInfo>;

    fn do_it(&self, input: &KV<String, Target>, emit: &mut dyn EmitFn<Self::Output>) {
        let target = &input.1;
        let mut entity = EntityInfo::new();
        entity.set_kind(EntityKind::E_TARGET);
        entity.set_file_type(FileType::BAZEL);
        entity.set_name(target.get_canonical_name().to_string());
        entity.set_file(target.get_filename().to_string());
        entity.set_line_number(target.get_line_number());
        entity
            .mut_keywords()
            .push(target.get_canonical_name().to_string());
        entity.mut_keywords().push(target.get_name().to_string());

        if target.get_files().len() > 0 {
            let mut subinfo = EntitySubInfo::new();
            subinfo.set_name(String::from("files"));
            for file in target.get_files() {
                subinfo.mut_item_texts().push(file.to_string());
                subinfo.mut_links().push(file.to_string());
            }
            entity.mut_subinfos().push(subinfo);
        }

        if target.get_dependencies().len() > 0 {
            let mut subinfo = EntitySubInfo::new();
            subinfo.set_name(String::from("dependencies"));
            for dep in target.get_dependencies() {
                subinfo.mut_item_texts().push(dep.to_string());
                subinfo.mut_links().push(dep.to_string());
            }
            entity.mut_subinfos().push(subinfo);
        }

        emit.emit(KV::new(entity.get_file().to_string(), entity));
    }
}

struct ExtractEntityInfoFromFilesFn {}
impl plume::DoFn for ExtractEntityInfoFromFilesFn {
    type Input = KV<String, File>;
    type Output = KV<String, EntityInfo>;

    fn do_it(&self, input: &KV<String, File>, emit: &mut dyn EmitFn<Self::Output>) {
        let file = &input.1;

        // Determine struct spans
        let mut struct_spans = Vec::new();
        for symbol in file.get_symbols() {
            if symbol.get_symbol_type() != SymbolType::STRUCTURE {
                continue;
            }

            if symbol.get_end_line_number() == 0 {
                continue;
            }

            struct_spans.push((
                symbol.get_symbol(),
                symbol.get_line_number(),
                symbol.get_end_line_number(),
            ));
        }
        struct_spans.sort_by_key(|x| x.1);

        for symbol in file.get_symbols() {
            if symbol.get_symbol_type() == SymbolType::VARIABLE {
                continue;
            }

            let mut container = None;
            if symbol.get_symbol_type() == SymbolType::FUNCTION {
                let idx =
                    match struct_spans.binary_search_by_key(&symbol.get_line_number(), |x| x.1) {
                        Ok(x) => x,
                        Err(x) => x,
                    };

                if idx == 0 {
                    continue;
                }

                if symbol.get_line_number() < struct_spans[idx - 1].2 {
                    container = Some(struct_spans[idx - 1].0);
                }
            }

            let mut entity = EntityInfo::new();

            if let Some(c) = container {
                let name = format!("{}::{}", c, symbol.get_symbol());
                entity.set_name(name.clone());
                entity.mut_keywords().push(name);
            } else {
                entity.set_name(symbol.get_symbol().to_string());
            }

            entity.mut_keywords().push(symbol.get_symbol().to_string());
            entity.set_file(file.get_filename().to_string());
            entity.set_line_number(symbol.get_line_number());
            entity.set_kind(match symbol.get_symbol_type() {
                SymbolType::VARIABLE => continue,
                SymbolType::FUNCTION => EntityKind::E_FUNCTION,
                SymbolType::STRUCTURE => EntityKind::E_STRUCT,
                SymbolType::TRAIT => EntityKind::E_TRAIT,
            });
            entity.set_file_type(file.get_file_type());

            emit.emit(KV::new(entity.get_file().to_string(), entity));
        }
    }
}

struct KeyEntitiesByKeywordFn {}
impl plume::DoFn for KeyEntitiesByKeywordFn {
    type Input = KV<String, EntityInfo>;
    type Output = KV<String, EntityInfo>;

    fn do_it(&self, input: &KV<String, EntityInfo>, emit: &mut dyn EmitFn<Self::Output>) {
        let entity = &input.1;
        for keyword in entity.get_keywords() {
            emit.emit(KV::new(
                search_utils::normalize_keyword(keyword),
                entity.clone(),
            ));
        }
    }
}

#[cfg(test)]
pub fn extract_entity_info_to_vec(
    targets: &PTable<String, Target>,
    files: &PTable<String, File>,
) -> (PTable<String, EntityInfo>, PTable<String, EntityInfo>) {
    let mut file_keyed_entities = PTable::concatenate(vec![
        targets.par_do(ExtractEntityInfoFromTargetsFn {}),
        files.par_do(ExtractEntityInfoFromFilesFn {}),
    ]);

    file_keyed_entities.write_to_vec();

    let mut keyword_keyed_entities = file_keyed_entities.par_do(KeyEntitiesByKeywordFn {});
    keyword_keyed_entities.write_to_vec();

    (file_keyed_entities, keyword_keyed_entities)
}

pub fn extract_and_write_entity_info(
    targets: &PTable<String, Target>,
    files: &PTable<String, File>,
    file_dest: &str,
    keyword_dest: &str,
) {
    let mut file_keyed_entities = PTable::concatenate(vec![
        targets.par_do(ExtractEntityInfoFromTargetsFn {}),
        files.par_do(ExtractEntityInfoFromFilesFn {}),
    ]);
    file_keyed_entities.write_to_sstable(file_dest);

    let mut keyword_keyed_entities = file_keyed_entities.par_do(KeyEntitiesByKeywordFn {});
    keyword_keyed_entities.write_to_sstable(keyword_dest);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_targets() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let mut t1 = Target::new();
        t1.set_name(String::from("review"));
        t1.set_canonical_name(String::from("//weld/review"));
        t1.mut_files().push(String::from("/abcdef/test.rs"));
        t1.mut_files().push(String::from("/abcdef/test_file.rs"));

        let p = PCollection::from_table(vec![KV::new(String::new(), t1)]);
        let f = PCollection::from_table(vec![]);
        let (mut fout, mut kout) = extract_entity_info_to_vec(&p, &f);

        plume::run();

        let entities = fout.into_vec();
        let kentities = kout.into_vec();

        assert_eq!(entities.len(), 1);
        assert_eq!(kentities.len(), 2);

        let entity = &entities[0].1;
        assert_eq!(entity.get_name(), String::from("//weld/review"));
        assert_eq!(entity.get_subinfos().len(), 1);

        assert_eq!(&kentities[0].0, "//weld/review");
        assert_eq!(&kentities[1].0, "review");

        let file_subinfo = &entity.get_subinfos()[0];
        assert_eq!(file_subinfo.get_name(), String::from("files"));
        assert_eq!(file_subinfo.get_item_texts().len(), 2);
    }
}
