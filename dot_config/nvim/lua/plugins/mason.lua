---@type LazySpec
return {
  -- use mason-tool-installer for automatically installing Mason packages
  {
    "WhoIsSethDaniel/mason-tool-installer.nvim",
    -- overrides `require("mason-tool-installer").setup(...)`
    opts = {
      -- Make sure to use the names found in `:Mason`
      ensure_installed = {
        -- Language servers from v4 configuration
        "lua-language-server", -- lua_ls in v4
        "eslint-lsp", -- eslint in v4
        "vue-language-server",
        "rust-analyzer", -- rust_analyzer in v4
        "css-lsp", -- cssls in v4
        "dockerfile-language-server", -- dockerls in v4
        "html-lsp", -- html in v4
        "json-lsp", -- jsonls in v4
        "texlab",
        "pyright",
        "svelte-language-server", -- svelte in v4
        "taplo",
        "tailwindcss-language-server", -- tailwindcss in v4
        "unocss-language-server", -- unocss in v4
        "yaml-language-server", -- yamlls in v4
        "bash-language-server", -- bashls in v4
        "prisma-language-server", -- prismals in v4
        "sqlls",
        "zls",

        -- Formatters and linters from v4 configuration
        "stylua",
        "yamlfmt",
        "jsonlint",
        "beautysh", -- corrected from "beatysh" in v4
        "sql-formatter",
        "shellcheck",
        "hadolint",
        "ruff",
        "rustfmt",
        "prettierd",

        -- Debuggers from v4 configuration
        "debugpy", -- python in v4

        -- Other tools
        "tree-sitter-cli",
      },
    },
  },

  -- Add any additional configuration needed for prettierd
  -- This handler code wasn't directly convertible to mason-tool-installer
  -- so it may need additional implementation
  -- {
  --   "nvimtools/none-ls.nvim",
  --   optional = true,
  --   opts = function(_, opts)
  --     local null_ls = require "null-ls"
  --     if null_ls and null_ls.builtins and null_ls.builtins.formatting then
  --       -- Add the conditional prettierd formatter
  --       table.insert(
  --         opts.sources or {},
  --         null_ls.builtins.formatting.prettierd.with {
  --           condition = function(utils)
  --             return utils.root_has_file(
  --               ".prettierrc",
  --               ".prettierrc.js",
  --               ".prettierrc.cjs",
  --               ".prettierrc.json",
  --               ".prettierrc.yaml",
  --               ".prettierrc.yml",
  --               ".prettierrc.toml"
  --             )
  --           end,
  --         }
  --       )
  --     end
  --     return opts
  --   end,
  -- },
}
