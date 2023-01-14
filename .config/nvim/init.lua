-- ensure the packer plugin manager is installed
local ensure_packer = function()
  local fn = vim.fn
  local install_path = fn.stdpath("data") .. "/site/pack/packer/start/packer.nvim"
  if fn.empty(fn.glob(install_path)) > 0 then
    fn.system({ "git", "clone", "--depth", "1", "https://github.com/wbthomason/packer.nvim", install_path })
    vim.cmd([[packadd packer.nvim]])
    return true
  end
  return false
end

local packer_bootstrap = ensure_packer()

require("packer").init({
  autoremove = true,
})
require('packer').startup(function(use)
  use 'wbthomason/packer.nvim' -- Package manager
  use { 'nvim-lua/plenary.nvim' }
  use { 'nvim-telescope/telescope.nvim', tag = '0.1.0' }
  use {
    'nvim-treesitter/nvim-treesitter',
    run = function()
      local ts_update = require('nvim-treesitter.install').update({ with_sync = true })
      ts_update()
    end,
  }
  use 'nvim-treesitter/playground'
  use { 'nvim-tree/nvim-web-devicons' }
  use({
    "j-hui/fidget.nvim",
    config = function()
      require("fidget").setup()
    end
  })

  -- Autocompletion framework
  use("hrsh7th/nvim-cmp")
  use({
    -- cmp LSP completion
    "hrsh7th/cmp-nvim-lsp",
    -- cmp Snippet completion
    "hrsh7th/cmp-vsnip",
    -- cmp Path completion
    "hrsh7th/cmp-path",
    "hrsh7th/cmp-buffer",
    after = { "hrsh7th/nvim-cmp" },
    requires = { "hrsh7th/nvim-cmp" },
  })

  -- Snippet engine
  use('hrsh7th/vim-vsnip')
  -- Adds extra functionality over rust analyzer
  use("rust-lang/rust.vim")
  use("simrat39/rust-tools.nvim")

  -- scheme 
  use 'folke/tokyonight.nvim'

  -- Scrollbar
  use("petertriho/nvim-scrollbar")

  -- Git sign
  use {
    'lewis6991/gitsigns.nvim',
    config = function()
      require('gitsigns').setup()
      require("scrollbar.handlers.gitsigns").setup()
    end
  }

  use {
    'nvim-lualine/lualine.nvim',
    requires = { 'kyazdani42/nvim-web-devicons', opt = true }
  }

  use {
    "tpope/vim-fugitive",
    opt = true,
    cmd = {
      "G", "Git", "Gdiffsplit", "Gvdiffsplit", "Gedit", "Gsplit",
      "Gread", "Gwrite", "Ggrep", "Glgrep", "Gmove",
      "Gdelete", "Gremove", "Gbrowse", "Gclog", "Gblame"
    },
  }
  use "idanarye/vim-merginal"
  use "tpope/vim-rhubarb"
  use "junegunn/gv.vim"

  use {
    "tpope/vim-eunuch",
  }

  use {
    'VonHeikemen/lsp-zero.nvim',
    requires = {
      -- LSP Support
      {'neovim/nvim-lspconfig'},
      {'williamboman/mason.nvim'},
      {'williamboman/mason-lspconfig.nvim'},

      -- Autocompletion
      {'hrsh7th/nvim-cmp'},
      {'hrsh7th/cmp-buffer'},
      {'hrsh7th/cmp-path'},
      {'saadparwaiz1/cmp_luasnip'},
      {'hrsh7th/cmp-nvim-lsp'},
      {'hrsh7th/cmp-nvim-lua'},

      -- Snippets
      {'L3MON4D3/LuaSnip'},
      {'rafamadriz/friendly-snippets'},
    }
  }

  use {
    'gelguy/wilder.nvim',
    config = function()
      -- config goes here
    end,
  }
end)

-- the first run will install packer and our plugins
if packer_bootstrap then
  require("packer").sync()
  return
end

require("tokyonight").setup({
  -- your configuration comes here
  -- or leave it empty to use the default settings
  style = "storm", -- The theme comes in three styles, `storm`, `moon`, a darker variant `night` and `day`
  light_style = "day", -- The theme is used when the background is set to light
  transparent = true, -- Enable this to disable setting the background color
  terminal_colors = true, -- Configure the colors used when opening a `:terminal` in Neovim
  styles = {
    -- Style to be applied to different syntax groups
    -- Value is any valid attr-list value for `:help nvim_set_hl`
    comments = { italic = true },
    keywords = { italic = true },
    functions = {},
    variables = {},
    -- Background styles. Can be "dark", "transparent" or "normal"
    sidebars = "transparent", -- style for sidebars, see below
    floats = "transparent", -- style for floating windows
  },
})
vim.cmd[[colorscheme tokyonight]]

-- Configuration
vim.wo.relativenumber = true
vim.wo.number = true

-- Telescope keybindings
local builtin = require('telescope.builtin')
vim.keymap.set('n', '<leader>ff', builtin.find_files, {})
vim.keymap.set('n', '<leader>fg', builtin.live_grep, {})
vim.keymap.set('n', '<leader>fb', builtin.buffers, {})
vim.keymap.set('n', '<leader>fh', builtin.help_tags, {})

-- Set completeopt to have a better completion experience
-- :help completeopt
-- menuone: popup even when there's only one match
-- noinsert: Do not insert text until a selection is made
-- noselect: Do not auto-select, nvim-cmp plugin will handle this for us.
vim.o.completeopt = "menuone,noinsert,noselect"

-- Avoid showing extra messages when using completion
vim.opt.shortmess = vim.opt.shortmess + "c"

-- Disable gitblame on startup
vim.g["gitblame_enabled"] = 0

local function on_attach(client, bufnr)
  -- Enable completion triggered by <c-x><c-o>
  vim.api.nvim_buf_set_option(bufnr, 'omnifunc', 'v:lua.vim.lsp.omnifunc')

  -- Mappings.
  -- See `:help vim.lsp.*` for documentation on any of the below functions
  local bufopts = { noremap=true, silent=true, buffer=bufnr }
  vim.keymap.set('n', 'gD', vim.lsp.buf.declaration, bufopts)
  vim.keymap.set('n', 'gd', vim.lsp.buf.definition, bufopts)
  vim.keymap.set('n', 'K', vim.lsp.buf.hover, bufopts)
  vim.keymap.set('n', 'gi', vim.lsp.buf.implementation, bufopts)
  vim.keymap.set('n', '<C-k>', vim.lsp.buf.signature_help, bufopts)
  vim.keymap.set('n', '<space>wa', vim.lsp.buf.add_workspace_folder, bufopts)
  vim.keymap.set('n', '<space>wr', vim.lsp.buf.remove_workspace_folder, bufopts)
  vim.keymap.set('n', '<space>wl', function()
    print(vim.inspect(vim.lsp.buf.list_workspace_folders()))
  end, bufopts)
  vim.keymap.set('n', '<space>D', vim.lsp.buf.type_definition, bufopts)
  vim.keymap.set('n', '<S-F6>', vim.lsp.buf.rename, bufopts)
  vim.keymap.set('n', '<space>ca', vim.lsp.buf.code_action, bufopts)
  vim.keymap.set('n', 'gr', vim.lsp.buf.references, bufopts)
  vim.keymap.set('n', '<space>f', function() vim.lsp.buf.format { async = true } end, bufopts)

end

-- Configure LSP through rust-tools.nvim plugin.
-- rust-tools will configure and enable certain LSP features for us.
-- See https://github.com/simrat39/rust-tools.nvim#configuration
local opts = {
  tools = {
    runnables = {
      use_telescope = true,
    },
    inlay_hints = {
      auto = true,
      show_parameter_hints = false,
      parameter_hints_prefix = "",
      other_hints_prefix = "",
    },
  },

  -- all the opts to send to nvim-lspconfig
  -- these override the defaults set by rust-tools.nvim
  -- see https://github.com/neovim/nvim-lspconfig/blob/master/CONFIG.md#rust_analyzer
  server = {
    -- on_attach is a callback called when the language server attachs to the buffer
    on_attach = on_attach,
    settings = {
      -- to enable rust-analyzer settings visit:
      -- https://github.com/rust-analyzer/rust-analyzer/blob/master/docs/user/generated_config.adoc
      ["rust-analyzer"] = {
        -- enable clippy on save
        checkOnSave = {
          command = "clippy",
        },
      },
    },
  },
}

vim.g["rustfmt_autosave"] = 1
require("rust-tools").setup(opts)

-- Treesitter configuration
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
    "query"
  },
  sync_install = false,
  auto_install = true,
  highlight = {
    enable = true,
    additional_vim_regex_highlighting = { "markdown" },
  },
}

-- scrollbar setup
local colors = require("tokyonight.colors").setup()
require("scrollbar").setup({
    handle = {
        color = colors.fg_gutter,
    },
    marks = {
        Search = { color = colors.orange },
        Error = { color = colors.error },
        Warn = { color = colors.warning },
        Info = { color = colors.info },
        Hint = { color = colors.hint },
        Misc = { color = colors.purple },
    }
})

-- lualine setup
require('lualine').setup {
  options = {
    theme = 'tokyonight'
  }
}

local kopts = {noremap = true, silent = true}

vim.api.nvim_create_autocmd("BufWritePre", {
  pattern = { "*.ts", "*.tsx", "*.vue", "*.js", "*.jsx" },
  command = "EslintFixAll"
})

-- Copy to clipboard
vim.keymap.set('v', '<leader>y', '"+y')
vim.keymap.set('n', '<leader>Y', '"+yg_')
vim.keymap.set('n', '<leader>y', '"+y')

-- Paste from clipboard
vim.keymap.set('n', '<leader>p', '"+p')
vim.keymap.set('n', '<leader>P', '"+P')
vim.keymap.set('v', '<leader>p', '"+p')
vim.keymap.set('v', '<leader>P', '"+P')

-- Vim diagnostic
vim.keymap.set('n', '<space>e', vim.diagnostic.open_float, kopts)
vim.keymap.set('n', '[d', vim.diagnostic.goto_prev, kopts)
vim.keymap.set('n', ']d', vim.diagnostic.goto_next, kopts)
vim.keymap.set('n', '<space>q', vim.diagnostic.setloclist, kopts)

-- Center after going up or down
vim.keymap.set('n', '<C-u>', '<C-u>zz')
vim.keymap.set('n', '<C-d>', '<C-d>zz')

-- Motion
vim.keymap.set('v', 'J', ":m '>+1<CR>gv=gv")
vim.keymap.set('v', 'K', ":m '<-2<CR>gv=gv")

-- Git
vim.keymap.set('n', 'gb', "<cmd>Git blame<cr>")
vim.keymap.set('n', 'gB', "<cmd>GitBlameToggle<cr>")
vim.keymap.set('n', 'gk', "<cmd>Git push<cr>")
vim.keymap.set('n', 'gK', "<cmd>Git pull<cr>")
vim.keymap.set('n', 'gw', "<cmd>cnext<cr>")
vim.keymap.set('n', 'gW', "<cmd>cprevious<cr>")
vim.keymap.set('n', '<leader>gB', '<cmd>MerginalToggle<cr>')

-- Line number color
-- vim.cmd('hi LineNr guibg=#24283b guifg=#ffffff')

vim.o.ma = true
vim.o.cursorline = true
vim.o.tabstop = 4
vim.o.shiftwidth = 4
vim.o.softtabstop = 4
vim.o.expandtab = true
vim.o.autoread = true
vim.o.nu = true
vim.o.foldlevelstart = 99
vim.o.scrolloff = 7
vim.o.backup = false
vim.o.writebackup = false
vim.o.swapfile = false

-- Wilder config
local wilder = require('wilder')
wilder.setup({modes = {':', '/', '?'}})
wilder.set_option('renderer', wilder.popupmenu_renderer({
  pumblend = 20,
  highlighter = wilder.basic_highlighter(),
  left = {' ', wilder.popupmenu_devicons()},
  right = {' ', wilder.popupmenu_scrollbar()},
}))

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
    'sumneko_lua',
    'pyright',
    'svelte',
    'taplo',
    'tailwindcss',
    'yamlls',
    'bashls'
})
lsp.setup()

-- source all the core config files
local core_conf_files = {
  "snippets.lua", -- snippets
}

for _, name in ipairs(core_conf_files) do
  local path = string.format("%s/core/%s", vim.fn.stdpath("config"), name)
  local source_cmd = "source " .. path
  vim.cmd(source_cmd)
end
