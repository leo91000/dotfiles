return {
	"AstroNvim/astrocore",
	---@type AstroCoreOpts
	opts = {
		mappings = {
			n = {
				["<leader>fw"] = {
					function()
						require("telescope.builtin").live_grep()
					end,
					desc = "Find words",
				},
				["<leader>fW"] = {
					function()
						require("telescope.builtin").live_grep({
							additional_args = function(args)
								return vim.list_extend(args, { "--hidden", "--no-ignore" })
							end,
						})
					end,
					desc = "Find words in all files",
				},
			},
		},
	},
}
