---@type LazySpec
return {
	-- use mason-lspconfig to configure LSP installations
	{
		"williamboman/mason-lspconfig.nvim",
		opts = function(_, opts)
			opts.ensure_installed = require("astrocore").list_insert_unique(opts.ensure_installed, {
				"eslint",
				"volar",
				"rust_analyzer",
				"cssls",
				"dockerls",
				"html",
				"jsonls",
				"texlab",
				"lua_ls",
				"pyright",
				"svelte",
				"taplo",
				"tailwindcss",
				"yamlls",
				"bashls",
				"prismals",
				"sqlls",
				"zls",
				"ts_ls",
			})
		end,
	},
	-- use mason-null-ls to configure Formatters/Linter installation for null-ls sources
	{
		"jay-babu/mason-null-ls.nvim",
		opts = function(_, opts)
			opts.ensure_installed = require("astrocore").list_insert_unique(opts.ensure_installed, {
				"stylua",
				"yamlfmt",
				"jsonlint",
				"beatysh",
				"sql-formatter",
				"shellcheck",
				"hadolint",
				"ruff",
				"rustfmt",
				"prettierd",
			})
		end,
	},
	{
		"jay-babu/mason-nvim-dap.nvim",
		opts = function(_, opts)
			opts.ensure_installed = require("astrocore").list_insert_unique(opts.ensure_installed, {
				"python",
			})
		end,
	},
}
