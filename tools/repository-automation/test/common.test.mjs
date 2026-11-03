import assert from 'node:assert/strict';
import test from 'node:test';
import { buildIssueBody, fingerprint, hasAssignableLabel, hasDuplicateSearchResult, isAssignmentRequest, redact } from '../lib/common.mjs';

test('redacts common token formats', () => {
  assert.match(redact('token=abc123 github_pat_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234'), /\[REDACTED\]/);
  assert.doesNotMatch(redact('token=abc123'), /abc123/);
});

test('assignment request recognition accepts explicit safe requests', () => {
  assert.equal(isAssignmentRequest('/assign'), true);
  assert.equal(isAssignmentRequest('I would like to work on this'), true);
  assert.equal(isAssignmentRequest('Nice work!'), false);
});

test('eligible labels are detected without contributor-history discrimination', () => {
  assert.equal(hasAssignableLabel([{ name: 'NSOC' }], ['NSOC']), true);
  assert.equal(hasAssignableLabel([{ name: 'bug' }], ['NSOC']), false);
});

test('generated issue bodies keep a stable duplicate marker', () => {
  const id = fingerprint('verified failure');
  const body = buildIssueBody({ fingerprint: id, name: 'Tests', command: ['cargo', 'test'], exitCode: 1, output: 'failed' });
  assert.match(body, new RegExp(`fingerprint=${id}`));
  assert.match(body, /Acceptance criteria/);
});

test('duplicate detection requires a matching issue or pull request', () => {
  assert.equal(hasDuplicateSearchResult({ total_count: 0 }), false);
  assert.equal(hasDuplicateSearchResult({ total_count: 1 }), true);
  assert.equal(hasDuplicateSearchResult({ total_count: '2' }), true);
  assert.equal(hasDuplicateSearchResult({}), false);
});
