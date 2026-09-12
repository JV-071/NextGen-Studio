-- Contract test for Studio integration. No game process or client assets required.
local request, environment, events, sequence = nil, nil, {}, 0
local loaded, destroyed, captured, responses = 0, 0, {}, {}
os.getenv = function(name) if name == 'NEXTGEN_STUDIO_SESSION_DIR' then return environment end end
io.open = function(path, mode)
  if mode == 'rb' then
    if not request then return nil end
    return {read=function() return request end, close=function() end}
  end
  return {write=function(_, text) responses[#responses+1]=text end, close=function() end}
end
json = {
  decode=function(raw)
    if raw=='invalid' then error('bad JSON') end
    return {revision=tonumber(raw),text=raw=='3' and 'broken' or 'Panel'}
  end,
  encode=function(value) return tostring(value.revision)..':'..value.message end
}
g_logger={info=function() end}
g_ui={
  getRootWidget=function() return {} end,
  loadUIFromString=function(text)
    if text=='broken' then error('parser rejected input') end
    loaded=loaded+1
    return {show=function() end,raise=function() end,destroy=function() destroyed=destroyed+1 end}
  end
}
g_app={doScreenshot=function(path) captured[#captured+1]=path end}
scheduleEvent=function(callback,delay) sequence=sequence+1;events[sequence]={callback=callback,delay=delay};return sequence end
removeEvent=function(id) events[id]=nil end
local function run(delay)
  for id,event in pairs(events) do
    if event.delay==delay then events[id]=nil;event.callback();return end
  end
  error('Expected scheduled event '..delay)
end
dofile('integration/dev_studio_bridge/bridge.lua')
init()
assert(next(events)==nil, 'normal client must not poll')
environment='/studio-session'
init()
request='1';run(250)
assert(loaded==1 and destroyed==0)
run(200)
assert(captured[1]=='/studio-session/preview.png')
run(250)
assert(loaded==1,'same revision must not reload')
request='2';run(250)
assert(loaded==2 and destroyed==1)
request='3';run(250)
assert(loaded==2 and destroyed==1,'failed load preserves prior preview')
request='invalid';run(250)
assert(responses[#responses]:find('bad JSON',1,true))
terminate()
assert(next(events)==nil and destroyed==2,'terminate removes timers and owned preview')
print('Preview integration contract passed')
