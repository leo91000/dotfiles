require'nvim-treesitter.configs'.setup {
  ensure_installed = {
    "help",
    "rust",
    "lua",
    "sql",
    "bash",
    "gitignore",
    "gitattributes",
    "javascript",
    "json",
    "json5",
    "markdown",
    "markdown_inline",
    "scss",
    "toml",
    "tsx",
    "typescript",
    "vim",
    "vue",
    "yaml",
    "query",
    "prisma",
    "sql",
  },
  sync_install = false,
  auto_install = true,
  highlight = {
    enable = true,
    disable = function(lang, bufnr)
      return 
        (lang == "cpp" and vim.api.nvim_buf_line_count(bufnr) > 50000) or
        (lang == "javascript" and vim.api.nvim_buf_line_count(bufnr) > 10000) or
        (lang == "typescript" and vim.api.nvim_buf_line_count(bufnr) > 10000)
    end,
    additional_vim_regex_highlighting = { "markdown" },
  },
}
