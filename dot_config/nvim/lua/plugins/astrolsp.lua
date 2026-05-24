local sep = package.config:sub(1, 1)

local function has_file(root_dir, filenames)
  if not root_dir or root_dir == "" then return false end
  local uv = vim.uv or vim.loop
  for _, filename in ipairs(filenames) do
    if uv.fs_stat(root_dir .. sep .. filename) then return true end
  end
  return false
end

return {
  "AstroNvim/astrolsp",
  opts = {
    features = {
      codelens = true,
      inlay_hints = false,
      semantic_tokens = true,
    },
    formatting = {
      format_on_save = {
        enabled = true,
        allow_filetypes = {},
        ignore_filetypes = {},
      },
      disabled = { "yamlls", "vtsls", "volar" },
      timeout_ms = 1000,
    },
    config = {
      eslint = {
        on_attach = function(client)
          if
            has_file(client.config.root_dir, {
              ".eslintrc",
              ".eslintrc.js",
              ".eslintrc.json",
              ".eslintrc.yaml",
              ".eslintrc.yml",
              "eslint.config.js",
              "eslint.config.mjs",
              "eslint.config.cjs",
            })
          then
            return
          end
          client:stop()
        end,
      },
      rust_analyzer = {
        settings = {
          ["rust-analyzer"] = {
            checkOnSave = true,
          },
        },
      },
      tailwindcss = {
        on_attach = function(client)
          if
            has_file(client.config.root_dir, {
              "tailwind.config.js",
              "tailwind.config.cjs",
              "tailwind.config.mjs",
              "tailwind.config.ts",
              "tailwind.config.cts",
              "tailwind.config.mts",
              "tailwind.config.json",
            })
          then
            return
          end
          client:stop()
        end,
      },
    },
    mappings = {
      n = {
        gl = { function() vim.diagnostic.open_float() end, desc = "Hover diagnostics" },
        gD = {
          function() vim.lsp.buf.declaration() end,
          desc = "Declaration of current symbol",
          cond = "textDocument/declaration",
        },
        ["<leader>dn"] = { function() vim.diagnostic.goto_next { wrap = true } end, desc = "Go to next diagnostic" },
        ["<leader>dp"] = { function() vim.diagnostic.goto_prev { wrap = true } end, desc = "Go to previous diagnostic" },
        ["<Leader>uY"] = {
          function() require("astrolsp.toggles").buffer_semantic_tokens() end,
          desc = "Toggle LSP semantic highlight (buffer)",
          cond = function(client) return client:supports_method "textDocument/semanticTokens/full" and vim.lsp.semantic_tokens ~= nil end,
        },
      },
    },
  },
}
