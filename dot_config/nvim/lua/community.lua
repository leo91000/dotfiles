---@type LazySpec
return {
	"AstroNvim/astrocommunity",
	{ import = "astrocommunity.pack.lua" },
	{ import = "astrocommunity.pack.rust" },
	-- { import = "astrocommunity.completion.copilot-lua" },
	{ import = "astrocommunity.completion.supermaven-nvim" },
	{ import = "astrocommunity.utility.telescope-live-grep-args-nvim" },
}
