return {
  "AstroNvim/astrocore",
  opts = {
    features = {
      large_buf = { size = 1024 * 500, lines = 10000 },
      autopairs = true,
      cmp = true,
      diagnostics = true,
      highlighturl = true,
      notifications = false,
    },
    diagnostics = {
      virtual_text = false,
      virtual_lines = false,
      update_in_insert = false,
      underline = true,
      severity_sort = true,
    },
    filetypes = {
      extension = {
        ejs = "ejs",
      },
    },
    options = {
      opt = {
        relativenumber = false,
        number = true,
        spell = false,
        signcolumn = "auto",
        wrap = false,
        scrolloff = 6,
        conceallevel = 2,
      },
      g = {},
    },
    treesitter = {
      ensure_installed = {
        "lua",
        "vim",
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
        "zig",
        "wgsl",
        "embedded_template",
      },
    },
    mappings = {
      n = {
        ["]b"] = { function() require("astrocore.buffer").nav(vim.v.count1) end, desc = "Next buffer" },
        ["[b"] = { function() require("astrocore.buffer").nav(-vim.v.count1) end, desc = "Previous buffer" },
        ["<Leader>bd"] = {
          function()
            require("astroui.status.heirline").buffer_picker(
              function(bufnr) require("astrocore.buffer").close(bufnr) end
            )
          end,
          desc = "Close buffer from tabline",
        },
        ["<Leader>b"] = { desc = "Buffers" },
      },
    },
  },
}
