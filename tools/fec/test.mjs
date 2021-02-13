export function assert_eq(left, right) {

  if (Array.isArray(left)) {
  }

  if (areEquivalent(right, left)) {
    return;
  }

  const leftStr = JSON.stringify(left, null, 4);
  const rightStr = JSON.stringify(right, null, 4);

  console.log(`assert_eq failed, provided: ${leftStr}\n\ndoes not equal expected   : ${rightStr}`);
  console.trace();
  throw 'assertion failed'
}

function isObject(obj) {
  return obj === Object(obj);
}

function areEquivalent(left, right) {
  if(Array.isArray(left)) {
    if(!Array.isArray(right)) return false;

    if(left.length !== right.length) return false;
    for (var i = 0; i < left.length; i++) {
      if (!areEquivalent(left[i], right[i])) return false;
    }
    
    return true;
  }

  if(isObject(left)) {
    if(!isObject(right)) return false;

    if (!areEquivalent(Object.keys(left), Object.keys(right))) {
      return false;
    }

    for(const key of Object.keys(left)) {
      if (!areEquivalent(left[key], right[key])) return false;
    }
    return true;
  }

  return left == right;
}

export function test(name, f) {
  console.log("--------------------------");
  console.log(`RUN "${name}"`);
  console.log("--------------------------");
  try {
    f()
  } catch(e) {
    console.log(`exception: ${e}`)
    console.log("--------------------------");
    console.log(`FAILED "${name}"`);
    console.log("--------------------------");
    process.exit(1);
  }
  console.log("--------------------------");
  console.log(`PASSED "${name}"`);
  console.log("--------------------------");
}
