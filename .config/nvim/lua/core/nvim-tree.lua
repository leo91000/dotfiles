-- disable netrw at the very start of your init.lua (strongly advised)
vim.g.loaded_netrw = 1
vim.g.loaded_netrwPlugin = 1

-- set termguicolors to enable highlight groups
vim.opt.termguicolors = true

require("nvim-tree").setup({
  disable_netrw = true,
  sort_by = "case_sensitive",
  view = {
    width = 30,
    mappings = {
      list = {
        { key = "u", action = "dir_up" },
      },
    },
  },
  actions = {
    open_file = {
      quit_on_open = true
    }
  },
  renderer = {
    group_empty = false,
  },
  filters = {
    dotfiles = true,
  },
  update_focused_file = {
    enable = true,
  },
})
