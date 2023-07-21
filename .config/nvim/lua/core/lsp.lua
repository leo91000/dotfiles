-- LSP Config
local cmp = require('cmp')
local lsp = require('lsp-zero')

lsp.preset({
  float_border = 'rounded',
  configure_diagnostics = true,
  setup_servers_on_start = true,
  call_servers = 'local',
  set_lsp_keymaps = false,
  manage_nvim_cmp = {
    set_sources = 'recommended',
    set_basic_mappings = true,
    set_extra_mappings = true,
    use_luasnip = true,
    set_format = true,
    documentation_window = true,
  },
})

vim.diagnostic.config({
    virtual_text = false,
})

lsp.ensure_installed({
    'eslint',
    'volar',
    'rust_analyzer',
    'cssls',
    'dockerls',
    'html',
    'jsonls',
    'texlab',
    'lua_ls',
    'pyright',
    'svelte',
    'taplo',
    'tailwindcss',
    'yamlls',
    'bashls',
    'prismals',
    'sqlls'
})

lsp.configure('tailwindcss', {
    filetypes = {
        "html",
        "javascriptreact",
        "typescriptreact",
        "rescript",
        "vue",
        "svelte",
        "rust"
    },
})

lsp.configure('volar', {
    filetypes = {
        "vue",
        "typescriptreact",
        "javascriptreact",
        "typescript",
        "javascript",
        "json"
    },
})

lsp.setup_nvim_cmp({
  sources = {
    {name = 'nvim_lsp', keyword_length = 3},
    {name = 'path'},
    {name = 'copilot'},
    {name = 'luasnip', keyword_length = 2},
    {name = 'buffer', keyword_length = 3},
  },
  mapping = lsp.defaults.cmp_mappings({
    ['<CR>'] = cmp.mapping.confirm({
      behavior = cmp.ConfirmBehavior.Replace,
      select = false,
    })
  })
})

lsp.setup()
