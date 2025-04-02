return {
  "catppuccin/nvim",
  name = "catppuccin",
  config = function()
    require("catppuccin").setup {
      flavour = "mocha", -- latte, frappe, macchiato, mocha
      term_colors = true,
      transparent_background = true,
      no_italic = false,
      no_bold = false,
      styles = {
        comments = {},
        conditionals = {},
        loops = {},
        functions = {},
        keywords = {},
        strings = {},
        variables = {},
        numbers = {},
        booleans = {},
        properties = {},
        types = {},
      },
      color_overrides = {
        mocha = { -- from my video sometime the mocha override does not kick in until aftill few restart
          base = "#000000",
          mantle = "#000000",
          crust = "#ffffff",
        },
      },
      highlight_overrides = {
        mocha = function(C)
          return {
            TabLineSel = { bg = C.pink },
            CmpBorder = { fg = C.surface2 },
            -- Pmenu = { bg = C.none },
            TelescopeBorder = { link = "FloatBorder" },
          }
        end,
      },
      integrations = {
        gitsigns = true,
        gitgutter = true,
        neotree = {
          enabled = true,
          show_root = true, -- makes the root folder not transparent
          transparent_panel = false, -- make the panel transparent
        },
        treesitter = true,
        notify = true,
        mason = true,
        barbecue = {
          dim_dirname = true, -- directory name is dimmed by default
          bold_basename = true,
          dim_context = false,
          alt_background = false,
        },
        -- For more plugins integrations please scroll down (https://github.com/catppuccin/nvim#integrations)
      },
    }
  end,
}
