local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.loop.fs_stat(lazypath) then
  vim.fn.system({
    "git",
    "clone",
    "--filter=blob:none",
    "https://github.com/folke/lazy.nvim.git",
    "--branch=stable", -- latest stable release
    lazypath,
  })
end
vim.opt.rtp:prepend(lazypath)

require('lazy').setup({
  {'nvim-lua/plenary.nvim'},
  {'nvim-telescope/telescope.nvim'},
  {
    'nvim-treesitter/nvim-treesitter',
    build = ":TSUpdate"
  },
  {'nvim-treesitter/playground'},

  -- UI 
  { 'nvim-tree/nvim-web-devicons' },
  {
    "j-hui/fidget.nvim",
    config = function()
      require("fidget").setup()
    end
  },
  'folke/tokyonight.nvim',
  "petertriho/nvim-scrollbar",
  {
    'lewis6991/gitsigns.nvim',
    config = function()
      require('gitsigns').setup()
      require("scrollbar.handlers.gitsigns").setup()
    end
  },
  {
    'nvim-lualine/lualine.nvim',
    requires = { 'kyazdani42/nvim-web-devicons', opt = true }
  },
  {
    'nvim-tree/nvim-tree.lua',
    requires = { 'nvim-tree/nvim-web-devicons' },
    tag = 'nightly'
  },
  {'gelguy/wilder.nvim'},

  -- GIT
  {"tpope/vim-fugitive"},
  {"idanarye/vim-merginal"},
  {"tpope/vim-rhubarb"},
  {"junegunn/gv.vim"},

  -- LSP Support
  {'neovim/nvim-lspconfig'},
  {'williamboman/mason.nvim'},
  {'williamboman/mason-lspconfig.nvim'},

  -- Autocompletion
  {'hrsh7th/nvim-cmp'},
  {'hrsh7th/cmp-nvim-lsp'},
  {'hrsh7th/cmp-buffer'},
  {'hrsh7th/cmp-path'},
  {'saadparwaiz1/cmp_luasnip'},
  {'hrsh7th/cmp-nvim-lua'},

  -- Snippets
  {'L3MON4D3/LuaSnip'},
  {'rafamadriz/friendly-snippets'},

  -- LSP zero preconfiguration
  {
    'VonHeikemen/lsp-zero.nvim',
    branch = 'v2.x',
    requires = {
      {'neovim/nvim-lspconfig'},
      {'williamboman/mason.nvim'},
      {'williamboman/mason-lspconfig.nvim'},
      {'hrsh7th/nvim-cmp'},
      {'hrsh7th/cmp-nvim-lsp'},
      {'hrsh7th/cmp-buffer'},
      {'hrsh7th/cmp-path'},
      {'saadparwaiz1/cmp_luasnip'},
      {'hrsh7th/cmp-nvim-lua'},
      {'L3MON4D3/LuaSnip'},
      {'rafamadriz/friendly-snippets'},
    }
  },

  -- Tools
  {"rust-lang/rust.vim"},
  {"simrat39/rust-tools.nvim"},
  {'gpanders/editorconfig.nvim'},
  {
    'numToStr/Comment.nvim',
    config = function()
      require('Comment').setup()
    end
  },
  {
    'zbirenbaum/copilot.lua',
    event = 'VimEnter',
    config = function()
      vim.defer_fn(function()
        require('copilot').setup()
      end, 100)
    end,
  },
  {
    'zbirenbaum/copilot-cmp',
    after = {'copilot.lua'},
    config = function ()
      require('copilot_cmp').setup()
    end
  },
  {
	"windwp/nvim-autopairs",
    config = function() require("nvim-autopairs").setup {} end
  },
  {
    'windwp/nvim-ts-autotag',
    config = function() require("nvim-ts-autotag").setup {} end
  },
  {
    "folke/which-key.nvim",
    config = function()
      vim.o.timeout = true
      vim.o.timeoutlen = 300
      require("which-key").setup({

      })
    end,
  },
  {
    'Wansmer/treesj',
    keys = { '<space>m', '<space>j', '<space>s' },
    dependencies = { 'nvim-treesitter/nvim-treesitter' },
    config = function()
      require('treesj').setup({
        max_join_length = 500,
      })
    end,
  }
})
