-- Hyprland configuration (Lua format, required from Hyprland 0.57 onward)

-- Monitors
hl.monitor({
    output = "desc:Ancor Communications Inc ROG PG279Q",
    mode = "2560x1440@144",
    position = "0x0",
    scale = 1,
})
hl.monitor({
    output = "desc:AOC AG274QG",
    mode = "2560x1440@240",
    position = "2560x0",
    scale = 1,
})
hl.monitor({
    output = "desc:Dell Inc. Dell AW2721D",
    mode = "2560x1440@144",
    position = "5120x0",
    scale = 1,
})
hl.monitor({
    output = "desc:AU Optronics 0xC5AC",
    mode = "3840x2400@120",
    position = "0x0",
    scale = 2,
})
hl.monitor({
    output = "",
    mode = "preferred",
    position = "auto",
    scale = "auto",
})

-- Environment
local environment = {
    XCURSOR_SIZE = "28",
    XDG_CURRENT_DESKTOP = "Hyprland",
    XDG_SESSION_TYPE = "wayland",
    XDG_SESSION_DESKTOP = "Hyprland",
    QT_QPA_PLATFORM = "wayland",
    QT_QPA_PLATFORMTHEME = "kde",
    QT_STYLE_OVERRIDE = "Breeze",
    QT_WAYLAND_DISABLE_WINDOWDECORATION = "1",
    BROWSER = "google-chrome-stable",
    ELECTRON_OZONE_PLATFORM_HINT = "wayland",
    LIBVA_DRIVER_NAME = "nvidia",
    __GLX_VENDOR_LIBRARY_NAME = "nvidia",
    GDK_SCALE = "1",
    GDK_DPI_SCALE = "1",
    QT_SCALE_FACTOR = "1",
}

for name, value in pairs(environment) do
    hl.env(name, value)
end

-- Startup
local startupCommands = {
    "/home/leoc/.config/hypr/scripts/start-hyprpaper-triple",
    "/home/leoc/.config/hypr/scripts/start-waybar-triple",
    "swaync",
    "ulauncher",
    "wl-paste --type text --watch cliphist store",
    "wl-paste --type image --watch cliphist store",
    "/usr/lib/hyprpolkitagent/hyprpolkitagent",
    "hyprctl setcursor default 28",
    "dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP",
    "systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP",
    "systemctl --user restart xdg-desktop-portal.service",
    "nm-applet",
    "1password",
    "protonvpn-app",
    "systemctl --user restart docker-desktop",
    "/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1",
}

hl.on("hyprland.start", function()
    for _, command in ipairs(startupCommands) do
        hl.exec_cmd(command)
    end
end)

-- Input, appearance, and behavior
hl.config({
    input = {
        kb_layout = "fr,fr",
        kb_variant = "nodeadkeys,",
        kb_options = "grp:menu_toggle",
        follow_mouse = 1,
        touchpad = {
            natural_scroll = true,
        },
    },
    general = {
        gaps_in = 4,
        gaps_out = 8,
        border_size = 2,
        col = {
            active_border = {
                colors = { "rgba(33ccffee)", "rgba(00ff99ee)" },
                angle = 45,
            },
            inactive_border = "rgba(595959aa)",
        },
        layout = "dwindle",
    },
    decoration = {
        rounding = 10,
        blur = {
            enabled = true,
            size = 8,
            passes = 3,
            new_optimizations = true,
        },
    },
    animations = {
        enabled = true,
    },
    misc = {
        render_unfocused_fps = 30,
    },
})

hl.curve("easeinout", {
    type = "bezier",
    points = { { 0.4, 0.0 }, { 0.2, 1.0 } },
})
hl.animation({ leaf = "windows", enabled = true, speed = 2, bezier = "easeinout" })
hl.animation({ leaf = "fade", enabled = true, speed = 3, bezier = "easeinout" })
hl.animation({ leaf = "border", enabled = true, speed = 3, bezier = "easeinout" })
hl.animation({ leaf = "workspaces", enabled = true, speed = 2, bezier = "easeinout" })

-- Keybinds
local mainMod = "SUPER"
local moveWindowEdgeThreshold = 24
local desktopMonitorSerials = {
    ["#ASN4JVmqxLLd"] = true,
    ["#GjMYMxgwABkB"] = true,
    ["#GrMYMxgwABcY"] = true,
}

local function restartMonitorSurfaces()
    hl.exec_cmd("pkill -x waybar || true")
    hl.exec_cmd("pkill -x hyprpaper || true")
    hl.exec_cmd("/home/leoc/.config/hypr/scripts/start-waybar-triple")
    hl.exec_cmd("/home/leoc/.config/hypr/scripts/start-hyprpaper-triple")
end

local function finishMonitorToggle(message)
    hl.dispatch(hl.dsp.focus({ monitor = "desc:AOC AG274QG" }))
    restartMonitorSurfaces()
    hl.notification.create({
        text = message,
        timeout = 3000,
    })
end

local function toggleMainMonitor()
    local activeDesktopMonitorCount = 0

    for _, monitor in ipairs(hl.get_monitors()) do
        if desktopMonitorSerials[monitor.serial] then
            activeDesktopMonitorCount = activeDesktopMonitorCount + 1
        end
    end

    if activeDesktopMonitorCount <= 1 then
        hl.monitor({
            output = "desc:AOC AG274QG",
            mode = "2560x1440@240",
            position = "2560x0",
            scale = 1,
            disabled = false,
        })
        hl.monitor({
            output = "desc:Ancor Communications Inc ROG PG279Q",
            mode = "2560x1440@144",
            position = "0x0",
            scale = 1,
            disabled = false,
        })
        hl.monitor({
            output = "desc:Dell Inc. Dell AW2721D",
            mode = "2560x1440@144",
            position = "5120x0",
            scale = 1,
            disabled = false,
        })
        hl.timer(function()
            finishMonitorToggle("Restored triple monitor setup")
        end, { timeout = 300, type = "oneshot" })
        return
    end

    hl.monitor({
        output = "desc:Ancor Communications Inc ROG PG279Q",
        disabled = true,
    })
    hl.monitor({
        output = "desc:Dell Inc. Dell AW2721D",
        disabled = true,
    })
    hl.timer(function()
        hl.monitor({
            output = "desc:AOC AG274QG",
            mode = "2560x1440@240",
            position = "0x0",
            scale = 1,
            disabled = false,
        })
        hl.timer(function()
            finishMonitorToggle("Using only the main monitor")
        end, { timeout = 300, type = "oneshot" })
    end, { timeout = 200, type = "oneshot" })
end

local function moveWindowSmart(direction)
    return function()
        local window = hl.get_active_window()
        if not window or not window.monitor then
            return
        end

        local monitor = window.monitor
        local windowPosition = window.at
        local windowSize = window.size
        local monitorWidth = monitor.width / monitor.scale
        local monitorHeight = monitor.height / monitor.scale
        local edgeDistances = {
            l = windowPosition.x - monitor.x,
            r = monitor.x + monitorWidth - (windowPosition.x + windowSize.x),
            u = windowPosition.y - monitor.y,
            d = monitor.y + monitorHeight - (windowPosition.y + windowSize.y),
        }

        if edgeDistances[direction] <= moveWindowEdgeThreshold then
            hl.dispatch(hl.dsp.window.move({ monitor = direction, window = window }))
            return
        end

        hl.dispatch(hl.dsp.window.move({ direction = direction, window = window }))
    end
end

local execBinds = {
    { mainMod .. " + Return", "ghostty" },
    { "ALT + space", "ulauncher-toggle" },
    { mainMod .. " + E", "thunar" },
    { mainMod .. " + F", "zen-browser" },
    { mainMod .. " + G", "google-chrome-stable" },
    { mainMod .. " + Z", "/home/leoc/.local/bin/zed" },
    { mainMod .. " + S", "steam" },
    { mainMod .. " + D", "discord" },
    { mainMod .. " + O", "obsidian" },
    { "CTRL + SHIFT + space", "1password --quick-access" },
    { mainMod .. " + SHIFT + S", "hyprshot -m region" },
    { mainMod .. " + SHIFT + M", "hyprshot -m output -m active" },
    { mainMod .. " + SHIFT + W", "hyprshot -m window -m active" },
    { "XF86AudioRaiseVolume", "wpctl set-volume -l 1.5 @DEFAULT_AUDIO_SINK@ 5%+" },
    { "XF86AudioLowerVolume", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-" },
    { "XF86AudioMute", "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle" },
    { "XF86AudioPlay", "playerctl play-pause" },
    { "XF86AudioNext", "playerctl next" },
    { "XF86AudioPrev", "playerctl previous" },
    { "XF86MonBrightnessDown", "brightnessctl set 5%-" },
    { "XF86MonBrightnessUp", "brightnessctl set 5%+" },
}

for _, bind in ipairs(execBinds) do
    hl.bind(bind[1], hl.dsp.exec_cmd(bind[2]))
end

hl.bind(mainMod .. " + Q", hl.dsp.window.close())
hl.bind(mainMod .. " + M", hl.dsp.exit())

local focusDirections = {
    H = "l",
    J = "d",
    K = "u",
    L = "r",
}

for key, direction in pairs(focusDirections) do
    hl.bind(mainMod .. " + " .. key, hl.dsp.focus({ direction = direction }))
end

hl.bind(mainMod .. " + SHIFT + K", hl.dsp.window.move({ direction = "u" }))
hl.bind(mainMod .. " + SHIFT + J", hl.dsp.window.move({ direction = "d" }))
hl.bind(mainMod .. " + SHIFT + H", moveWindowSmart("l"))
hl.bind(mainMod .. " + SHIFT + L", moveWindowSmart("r"))
hl.bind(mainMod .. " + SHIFT + O", toggleMainMonitor)
hl.bind(mainMod .. " + F11", hl.dsp.window.fullscreen())
hl.bind(mainMod .. " + Space", hl.dsp.window.float())
hl.bind(mainMod .. " + T", hl.dsp.layout("togglesplit"))
hl.bind(mainMod .. " + P", hl.dsp.window.pseudo())

local workspaceKeys = {
    "ampersand",
    "eacute",
    "quotedbl",
    "apostrophe",
    "parenleft",
    "minus",
    "egrave",
    "underscore",
    "ccedilla",
}

for workspace, key in ipairs(workspaceKeys) do
    hl.bind(mainMod .. " + " .. key, hl.dsp.focus({ workspace = workspace }))
    hl.bind(mainMod .. " + SHIFT + " .. key, hl.dsp.window.move({ workspace = workspace }))
end

-- Window rules
hl.window_rule({
    name = "jetbrains-noinitialfocus",
    match = {
        class = "^jetbrains-.*",
        initial_class = "negative:^jetbrains-toolbox$",
        float = true,
    },
    no_initial_focus = true,
})

hl.window_rule({
    name = "jetbrains-popups",
    match = {
        class = "^jetbrains-.*",
        initial_class = "negative:^jetbrains-toolbox$",
        title = "negative:^win.*",
        float = true,
    },
    move = { "30%", "30%" },
    size = { "40%", "40%" },
})

hl.window_rule({
    name = "steam-suppress-maximize",
    match = { class = "^([Ss]team|[Ss]teamwebhelper)$" },
    suppress_event = "maximize fullscreen",
})

hl.window_rule({
    name = "wartales-background",
    match = { class = "^steam_app_1527950$" },
    render_unfocused = true,
})
