-- LSP Config
local cmp = require('cmp')
local lsp = require('lsp-zero')

lsp.preset('recommended', {
    name = 'minimal',
    set_lsp_keymaps = false,
})

lsp.ensure_installed({
    'tsserver',
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

lsp.configure('unocss', {
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
      -- documentation says this is important.
      -- I don't know why.
      behavior = cmp.ConfirmBehavior.Replace,
      select = false,
    })
  })
})


lsp.setup()
