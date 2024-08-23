---@type LazySpec
return {
  "nvim-treesitter/nvim-treesitter",
  opts = function(_, opts)
    opts.ensure_installed = require("astrocore").list_insert_unique(opts.ensure_installed, {
      "lua",
      "python",
      "javascript",
      "typescript",
      "html",
      "css",
      "json",
      "yaml",
      "toml",
      "bash",
      "tsx",
      "vue",
      "svelte",
      "prisma",
      "svelte",
      "zig",
      "wgsl",
    })
  end,
}
