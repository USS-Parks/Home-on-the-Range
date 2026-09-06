'use strict';
const assert = require('node:assert/strict');
const allowed = new Set(['health','search','get','create','revise'].map(name => 'mcp__hotr__hotr_'+name));

function decode(row) {
  let text = row.result;
  assert.equal(typeof text, 'string');
  const opening = '<untrusted_tool_result source="'+row.name+'">\n';
  const closing = '\n</untrusted_tool_result>';
  if (text.startsWith(opening)) {
    assert.ok(text.endsWith(closing), 'Incomplete native untrusted-result wrapper');
    const boundary = text.indexOf('\n\n', opening.length);
    assert.ok(boundary >= opening.length, 'Missing native wrapper boundary');
    text = text.slice(boundary + 2, -closing.length).trim();
  }
  return JSON.parse(text);
}

function normalize(calls, results) {
  assert.ok(calls.length > 0 && calls.length <= 8, 'Native call limit');
  assert.equal(calls.length, results.length, 'Missing native tool result');
  const byId = new Map(results.map(row => [row.id, row]));
  assert.equal(byId.size, results.length, 'Duplicate native result ID');
  const resolved = [], toolResults = [], discovery = [], ids = new Set();
  for (const call of calls) {
    assert.ok(typeof call.id === 'string' && !ids.has(call.id), 'Invalid native call ID');
    ids.add(call.id);
    const name = call.function?.name || call.name;
    const args = JSON.parse(call.function?.arguments || '{}');
    const result = byId.get(call.id);
    assert.ok(result, 'Native result ID does not match call');
    if (name === 'tool_search' || name === 'tool_describe') {
      assert.equal(result.name, name);
      const catalog = decode(result);
      assert.ok(catalog.tools && Object.keys(catalog.tools).length > 0);
      assert.ok(Object.keys(catalog.tools).every(tool => allowed.has(tool)), 'Discovery escaped HOTR catalog');
      if (name === 'tool_search') {
        assert.equal(catalog.total_available, 5, 'Discovery catalog was broader than HOTR');
        assert.ok(catalog.results.every(entry => entry.matches.every(tool => allowed.has(tool))));
      }
      discovery.push({id:call.id,name});
      continue;
    }
    const target = name === 'tool_call' ? args.name : name;
    assert.ok(allowed.has(target), 'Native dispatch targeted a non-HOTR tool');
    assert.equal(result.name, target, 'Native result resolved a different tool');
    resolved.push({...call,transport_tool:name,function:{
      name:target,arguments:name === 'tool_call' ? JSON.stringify(args.arguments) : call.function.arguments,
    }});
    toolResults.push({...result,result:decode(result)});
  }
  assert.ok(resolved.length > 0, 'No native HOTR operation');
  return {calls:resolved,tool_results:toolResults,discovery_calls:discovery};
}
module.exports = {normalize};
