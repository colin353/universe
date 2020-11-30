pub fn get_block(
    index: &sstable_proto_rust::Index,
    key: &str,
) -> Option<sstable_proto_rust::KeyEntry> {
    match _get_block_index(index, key, false, false) {
        Some(i) => Some(index.pointers[i].to_owned()),
        None => None,
    }
}

pub fn get_shard_boundaries(
    index: &sstable_proto_rust::Index,
    target_shard_count: usize,
) -> Vec<String> {
    if target_shard_count <= 1 {
        return Vec::new();
    }

    let num_pointers = index.get_pointers().len();
    let mut output = Vec::new();
    if num_pointers < target_shard_count {
        for pointer in index.get_pointers() {
            output.push(pointer.get_key().to_string());
        }
        return output;
    }

    for i in 1..target_shard_count {
        let t = ((i * num_pointers) / target_shard_count) as usize;
        let ref keyentry = index.pointers[t];
        output.push(keyentry.get_key().to_string());
    }

    output
}

// Suggest possible sharding points based on the contents of the index. The suggested sharding
// points will be roughly of equal size. You should prefix and suffix with the  min and max
// keys, then the suggested shards should be composed of the intervals between the resulting
// keys. When using a key_spec, an implicit final shard should be included from the last key to
// the end of the keyspec.
pub fn suggest_shards(
    index: &sstable_proto_rust::Index,
    key_spec: &str,
    min_key: &str,
    max_key: &str,
) -> Vec<String> {
    let maybe_index: Option<usize>;
    let lower_bound = if key_spec > min_key {
        // In this case, we will use the key_spec to retrieve the block.
        maybe_index = get_block_index_with_keyspec(index, key_spec);
        key_spec
    } else {
        maybe_index = get_block_index_with_min_key(index, min_key);
        min_key
    };

    let mut boundaries = Vec::new();
    let mut sample_rate = 1;
    let mut count = 0;

    // Find the SSTable boundaries within the spec.
    if let Some(idx) = maybe_index {
        for i in (idx as usize)..index.get_pointers().len() {
            let ref keyentry = index.pointers[i];

            // Make sure we have passed the min key.
            if keyentry.get_key() < lower_bound {
                continue;
            }

            // If we have passed the key spec, quit.
            if key_spec != "" && !keyentry.get_key().starts_with(key_spec) {
                break;
            }

            // If we have passed the max key, quit.
            if max_key != "" && keyentry.get_key() > max_key {
                break;
            }

            // If we start collecting loads of keys, downsample the amount we extract.
            // Arbitrarily start downsampling after extracting 1<<6 samples, which is 64.
            if boundaries.len() > (sample_rate << 6) {
                sample_rate *= 2;
            }

            if count % sample_rate == 0 {
                boundaries.push(keyentry.get_key().to_owned());
            }

            count += 1;
        }
    }

    boundaries
}

pub fn get_block_with_keyspec(
    index: &sstable_proto_rust::Index,
    key_spec: &str,
) -> Option<sstable_proto_rust::KeyEntry> {
    match _get_block_index(index, key_spec, true, false) {
        Some(i) => Some(index.pointers[i].to_owned()),
        None => None,
    }
}

// If we have a minimum key, jump to the first record either equal to or greater than the key.
pub fn get_block_with_min_key(
    index: &sstable_proto_rust::Index,
    min_key: &str,
) -> Option<sstable_proto_rust::KeyEntry> {
    match _get_block_index(index, min_key, false, true) {
        Some(i) => Some(index.pointers[i].to_owned()),
        None => None,
    }
}

pub fn get_block_index_with_keyspec(
    index: &sstable_proto_rust::Index,
    key_spec: &str,
) -> Option<usize> {
    _get_block_index(index, key_spec, true, false)
}

pub fn get_block_index_with_min_key(
    index: &sstable_proto_rust::Index,
    min_key: &str,
) -> Option<usize> {
    _get_block_index(index, min_key, false, true)
}

// _get_block searches the index for a possible key. If a suitable block is found, it'll
// return the byte offset for that block.
fn _get_block_index(
    index: &sstable_proto_rust::Index,
    key: &str,
    as_key_spec: bool,
    as_min_key: bool,
) -> Option<usize> {
    let pointers = index.get_pointers();
    let length = pointers.len();
    if length == 0 {
        return None;
    }

    // First, we must find out the number of bits in the number.
    let mut bit_index = 1;
    while (length >> bit_index) > 0 {
        bit_index += 1
    }
    let mut i = 0;
    while bit_index > 0 {
        bit_index -= 1;
        i += 1 << bit_index;

        if i >= length || pointers[i].get_key() > key {
            // Unset the bit in question: we've gone too far down the list.
            i -= 1 << bit_index;
        } else if pointers[i].get_key() < key {
            // Do nothing, since we haven't gone far enough.
        } else {
            return Some(i);
        }
    }

    // For the case of using a key_spec, the key_spec is expected to rank higher than any value
    // fulfilling the spec. Therefore we may observe that the block key is higher than the
    // key_spec, which is acceptable as long as the block key matches the key spec.
    let allowable_for_key_spec = as_key_spec && (pointers[i].get_key().starts_with(key));

    match pointers[i].get_key() <= key || allowable_for_key_spec || as_min_key {
        true => Some(i),

        // If the block we found starts with a key which is already
        // higher than our key, that means our key doesn't exist.
        false => None,
    }
}
