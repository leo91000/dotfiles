require("lazy").setup({
	{
		"AstroNvim/AstroNvim",
		version = "^4",
		import = "astronvim.plugins",
		opts = { -- AstroNvim options must be set here with the `import` key
			mapleader = " ",
			maplocalleader = ",",
			icons_enabled = true,
			pin_plugins = nil,
		},
	},
	{ import = "community" },
	{ import = "plugins" },
} --[[@as LazySpec]], {
	-- Configure any other `lazy.nvim` configuration options here
	install = { colorscheme = { "catpuccin" } },
	ui = { backdrop = 100 },
	performance = {
		rtp = {
			-- disable some rtp plugins, add more to your liking
			disabled_plugins = {
				"gzip",
				"netrwPlugin",
				"tarPlugin",
				"tohtml",
				"zipPlugin",
			},
		},
	},
} --[[@as LazyConfig]])
