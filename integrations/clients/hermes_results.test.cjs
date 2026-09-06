'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const {normalize} = require('./hermes_results.cjs');
const name = 'mcp__hotr__hotr_get';
const call = {id:'actual-1',function:{name:'tool_call',arguments:JSON.stringify({name,arguments:{namespace:'demo',id:'colour'}})}};
const result = {id:call.id,name,result:'<untrusted_tool_result source="'+name+'">\nNative untrusted-data notice.\n\n'+JSON.stringify({structuredContent:{revision:3,body:'green'}})+'\n</untrusted_tool_result>'};

test('native router identity and wrapped tool result remain independently paired', () => {
  const actual = normalize([call],[result]);
  assert.equal(actual.calls[0].function.name,name);
  assert.equal(actual.calls[0].transport_tool,'tool_call');
  assert.equal(actual.tool_results[0].result.structuredContent.revision,3);
});
test('native metadata discovery is allowed only for the five-tool HOTR catalog', () => {
  const discovery = {id:'discovery',function:{name:'tool_search',arguments:'{}'}};
  const catalog = {id:'discovery',name:'tool_search',result:JSON.stringify({
    total_available:5,tools:{[name]:{}},results:[{matches:[name]}],
  })};
  const actual = normalize([discovery,call],[catalog,result]);
  assert.equal(actual.calls.length,1);
  assert.equal(actual.discovery_calls.length,1);
  catalog.result = JSON.stringify({total_available:6,tools:{terminal:{}},results:[]});
  assert.throws(() => normalize([discovery,call],[catalog,result]));
});
test('foreign dispatch, mismatched identity, duplicate IDs and broken wrappers fail closed', () => {
  assert.throws(() => normalize([{...call,function:{name:'tool_call',arguments:'{"name":"terminal","arguments":{}}'}}],[result]));
  assert.throws(() => normalize([call],[{...result,name:'mcp__other__get'}]));
  assert.throws(() => normalize([call],[{...result,id:'unmatched'}]));
  assert.throws(() => normalize([call,call],[result,result]));
  assert.throws(() => normalize([call],[{...result,result:result.result.replace('</untrusted_tool_result>','')}]));
});
