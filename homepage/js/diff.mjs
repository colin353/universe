export function diff(file1, file2) {
    var equivalenceClasses = {};
    for (var j = 0; j < file2.length; j++) {
	var line = file2[j];
	if (equivalenceClasses[line]) {
	    equivalenceClasses[line].push(j);
	} else {
	    equivalenceClasses[line] = [j];
	}
    }

    var candidates = [{file1index: -1,
		       file2index: -1,
		       chain: null}];

    for (var i = 0; i < file1.length; i++) {
	var line = file1[i];
	var file2indices = equivalenceClasses[line] || [];

	var r = 0;
	var c = candidates[0];

	for (var jX = 0; jX < file2indices.length; jX++) {
	    var j = file2indices[jX];

	    for (var s = 0; s < candidates.length; s++) {
		if ((candidates[s].file2index < j) &&
		    ((s == candidates.length - 1) ||
		     (candidates[s + 1].file2index > j)))
		    break;
	    }

	    if (s < candidates.length) {
		var newCandidate = {file1index: i,
				    file2index: j,
				    chain: candidates[s]};
		if (r == candidates.length) {
		    candidates.push(c);
		} else {
		    candidates[r] = c;
		}
		r = s + 1;
		c = newCandidate;
		if (r == candidates.length) {
		    break; // no point in examining further (j)s
		}
	    }
	}

	candidates[r] = c;
    }

    // At this point, we know the LCS: it's in the reverse of the
    // linked-list through .chain of
    // candidates[candidates.length - 1].

    // We now apply the LCS to build a "comm"-style picture of the
    // differences between file1 and file2.

    var result = [];
    var tail1 = file1.length;
    var tail2 = file2.length;
    var common = {common: []};

    function processCommon() {
	if (common.common.length) {
	    common.common.reverse();
	    result.push(common);
	    common = {common: []};
	}
    }

    for (var candidate = candidates[candidates.length - 1];
	 candidate != null;
	 candidate = candidate.chain) {
	var different = {file1: [], file2: []};

	while (--tail1 > candidate.file1index) {
	    different.file1.push(file1[tail1]);
	}

	while (--tail2 > candidate.file2index) {
	    different.file2.push(file2[tail2]);
	}

	if (different.file1.length || different.file2.length) {
	    processCommon();
	    different.file1.reverse();
	    different.file2.reverse();
	    result.push(different);
	}

	if (tail1 >= 0) {
	    common.common.push(file1[tail1]);
	}
    }

    processCommon();

    result.reverse();
    return result;
}
