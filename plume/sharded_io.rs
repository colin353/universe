use plume_proto_rust::*;
use shard_lib;

enum ShardingStrategy {
    Any,
    ExclusiveKeyRange,
}

fn shard(stage: &Stage, strategy: ShardingStrategy, target_shards: usize) -> Vec<Shard> {
    Vec::new()
}

fn shard_inputs(
    input: &PCollectionProto,
    strategy: ShardingStrategy,
    target_shards: usize,
) -> Vec<PCollectionProto> {
    // RecordIO doesn't support keyrange sharding, so we have to just use whatever sharding
    // strategy was present on the input.
    if input.get_format() == DataFormat::RECORDIO {
        return shard_lib::unshard(input.get_filename())
            .iter()
            .map(|f| {
                let mut s = input.clone();
                s.set_filename(f.to_string());
                s
            })
            .collect();
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_recordio() {
        let mut input = PCollectionProto::new();
        input.set_format(DataFormat::RECORDIO);
        input.set_filename(String::from("/tmp/data.recordio@2"));
        assert_eq!(
            shard_inputs(&input, ShardingStrategy::Any, 10)
                .iter()
                .map(|x| x.get_filename())
                .collect::<Vec<_>>(),
            vec![
                "/tmp/data.recordio-00000-of-00002",
                "/tmp/data.recordio-00001-of-00002",
            ]
        );
    }
}
