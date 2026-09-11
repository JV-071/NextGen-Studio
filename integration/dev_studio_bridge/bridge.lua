-- Original NextGen Studio integration. No socket, online-mode emulation or global overrides.
local timer
local preview
local session
local revision
local active = false

local function report(message)
  g_logger.info('[NextGen Studio] ' .. tostring(message))
end

local function poll()
  if not active then return end
  local ok, err = pcall(function()
    local raw = g_resources.readFileContents('/dev_studio_bridge/request.json')
    if not raw or #raw > 32768 then return end
    local request = json.decode(raw)
    if type(request) ~= 'table' or request.token ~= session or
       type(request.revision) ~= 'string' or request.revision == revision then return end
    revision = request.revision
    local path = request.path
    if type(path) ~= 'string' or path:sub(1, 1) ~= '/' or
       path:find('..', 1, true) or path:find('\\', 1, true) or
       not path:match('%.otui$') then
      report('Caminho de interface invalido.')
      return
    end
    local loaded, widget = pcall(g_ui.loadUI, path, g_ui.getRootWidget())
    if not loaded or not widget then
      report('Falha ao abrir interface: ' .. tostring(widget))
      return
    end
    if preview then preview:destroy() end
    preview = widget
    preview:show()
    preview:raise()
    report('Interface carregada: ' .. path)
  end)
  if not ok then report(err) end
  if active then timer = scheduleEvent(poll, 500) end
end

function init()
  session = os.getenv('NEXTGEN_STUDIO_SESSION')
  if not session or #session < 16 then return end
  active = true
  report('Integracao ativa. Scripts do projeto serao executados na previa.')
  timer = scheduleEvent(poll, 250)
end

function terminate()
  active = false
  if timer then removeEvent(timer); timer = nil end
  if preview then preview:destroy(); preview = nil end
  revision = nil
end
