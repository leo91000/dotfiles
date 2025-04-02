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
				-- Git
				["<leader>gk"] = {
					function()
						vim.cmd("Git push")
					end,
					desc = "Git push",
				},
				["<leader>gF"] = {
					function()
						vim.cmd("Git push --force-with-lease")
					end,
					desc = "Git push (force with lease)",
				},
				["<leader>gK"] = {
					function()
						vim.cmd("Git pull")
					end,
					desc = "Git pull --rebase",
				},

				-- Scroll
				["<C-e>"] = { "<C-e>zz", desc = "Scroll down 1 line" },
				["<C-y>"] = { "<C-y>zz", desc = "Scroll up 1 line" },
				["<C-d>"] = { "<C-d>zz", desc = "Scroll down 1/2 page" },
				["<C-u>"] = { "<C-u>zz", desc = "Scroll up 1/2 page" },
				["<C-f>"] = { "<C-f>zz", desc = "Scroll down 1 page" },
				["<C-b>"] = { "<C-b>zz", desc = "Scroll up 1 page" },

				-- navigate buffer tabs with `H` and `L`
				L = {
					"<cmd>bnext<cr>",
					desc = "Next buffer",
				},
				H = {
					"<cmd>bprev<cr>",
					desc = "Previous buffer",
				},

				-- mappings seen under group name "Buffer"
				["<leader>bD"] = {
					function()
						require("astronvim.utils.status").heirline.buffer_picker(function(bufnr)
							require("astronvim.utils.buffer").close(bufnr)
						end)
					end,
					desc = "Pick to close",
				},

				-- tables with the `name` key will be registered with which-key if it's installed
				-- this is useful for naming menus
				["<leader>b"] = { name = "Buffers" },

				-- quick save
				-- ["<C-s>"] = { ":w!<cr>", desc = "Save File" },  -- change description but the same command

				["<F9>"] = { ":LspRestart<cr>", desc = "Restart LSPs" },

				["<leader>m"] = {
					function()
						require("treesj").toggle()
					end,
					desc = "Toggle treesj",
				},
			},
			i = {
				["<C-h>"] = { "<C-w>", desc = "Delete word" },
				["<C-Del>"] = { "<C-o>dw", desc = "Delete next word" },
			},
		},
	},
}
