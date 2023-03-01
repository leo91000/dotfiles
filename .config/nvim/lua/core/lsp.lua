-- LSP Config
local lsp = require('lsp-zero')
lsp.preset('recommended')

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

lsp.setup()
