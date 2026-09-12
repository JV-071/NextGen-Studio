-- NextGen Studio Rust: isolated process, live unsaved OTUI and on-demand capture.
local timer, captureTimer, preview, session, revision
local active = false
local function read(name)
  local f = io.open(session .. '/' .. name, 'rb')
  if not f then return nil end
  local data = f:read(8388609)
  f:close()
  if data and #data <= 8388608 then return data end
end
local function report(message)
  g_logger.info('[NextGen Studio] ' .. tostring(message))
  local f = io.open(session .. '/response.json', 'wb')
  if f then f:write(json.encode({revision = revision, message = tostring(message)})); f:close() end
end
local function poll()
  if not active then return end
  local ok, err = pcall(function()
    local raw = read('request.json')
    if not raw then return end
    local request = json.decode(raw)
    if type(request) ~= 'table' or type(request.revision) ~= 'number' or
       request.revision == revision or type(request.text) ~= 'string' then return end
    revision = request.revision
    if captureTimer then removeEvent(captureTimer); captureTimer = nil end
    local loaded, widget = pcall(g_ui.loadUIFromString, request.text, g_ui.getRootWidget())
    if not loaded or not widget then report('Falha na previa: ' .. tostring(widget)); return end
    if preview then preview:destroy() end
    preview = widget
    preview:show()
    preview:raise()
    report('Revisao ' .. revision .. ' carregada pelo motor NextGen.')
    captureTimer = scheduleEvent(function()
      captureTimer = nil
      if active then g_app.doScreenshot(session .. '/preview.png') end
    end, 200)
  end)
  if not ok then report(err) end
  if active then timer = scheduleEvent(poll, 250) end
end
function init()
  session = os.getenv('NEXTGEN_STUDIO_SESSION_DIR')
  if not session or session == '' then return end
  active = true
  timer = scheduleEvent(poll, 250)
end
function terminate()
  active = false
  if timer then removeEvent(timer); timer = nil end
  if captureTimer then removeEvent(captureTimer); captureTimer = nil end
  if preview then preview:destroy(); preview = nil end
end
