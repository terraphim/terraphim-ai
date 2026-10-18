'use strict';
// Full serialized state matters for annotation-only operations: body and KG
// can already match while the issued native command/Save is still pending.
function matchesSavedSource(actual, expected) {
  if (typeof actual !== 'string' || (expected !== undefined && typeof expected !== 'string')) {
    throw new TypeError('Saved source and optional expected snapshot must be strings');
  }
  return expected === undefined || actual === expected;
}
module.exports = { matchesSavedSource };
