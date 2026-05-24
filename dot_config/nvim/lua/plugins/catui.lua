return {
  "catppuccin/nvim",
  name = "catppuccin",
  config = function()
    require("catppuccin").setup {
      flavour = "mocha",
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
        mocha = {
          base = "#000000",
          mantle = "#000000",
          crust = "#ffffff",
        },
      },
      highlight_overrides = {
        mocha = function(colors)
          return {
            TabLineSel = { bg = colors.pink },
            CmpBorder = { fg = colors.surface2 },
            TelescopeBorder = { link = "FloatBorder" },
          }
        end,
      },
      integrations = {
        gitsigns = true,
        gitgutter = true,
        neotree = {
          enabled = true,
          show_root = true,
          transparent_panel = false,
        },
        treesitter = true,
        notify = true,
        mason = true,
        barbecue = {
          dim_dirname = true,
          bold_basename = true,
          dim_context = false,
          alt_background = false,
        },
      },
    }
  end,
}
