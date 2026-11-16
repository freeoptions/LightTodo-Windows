const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');

const source = ts.transpileModule(
  fs.readFileSync('src/utils/memoFormat.ts', 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } },
).outputText;
const memo = {};
vm.runInNewContext(source, {
  exports: memo,
  document: {
    createElement() { throw new Error('Legacy text must never be parsed as HTML'); },
  },
});

test('existing Chinese notes preserve line breaks, spacing and URLs', () => {
  const text = '  未提测的\nhttps://example.com/story-view-15479.html\n\n【仓储】虚拟出入库\n';
  assert.equal(memo.decodeMemo(text).text, text);
  assert.equal(memo.decodeMemo('').text, '');
});

test('HTML-like legacy notes remain literal text', () => {
  const text = '<b>这不是旧数据里的加粗</b>\n<script>alert(1)</script>';
  assert.equal(memo.decodeMemo(text).text, text);
  assert.equal(memo.decodeMemo(text).html, undefined);
});

test('invalid or incomplete format envelopes preserve the original content', () => {
  for (const suffix of ['{', 'null', '42', '[]', '{}', '{"html":12}']) {
    const text = 'lighttodo:rich-memo:v1:' + suffix;
    assert.equal(memo.decodeMemo(text).text, text);
  }
});

test('unknown format versions remain readable as text', () => {
  const text = 'lighttodo:rich-memo:v2:{"html":"<b>保留原文</b>"}';
  assert.equal(memo.decodeMemo(text).text, text);
});
