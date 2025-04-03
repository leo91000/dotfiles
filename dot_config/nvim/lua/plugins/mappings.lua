return {
	"AstroNvim/astrocore",
	---@type AstroCoreOpts
	opts = {
		mappings = {
			n = {
				-- Git operations
				["<Leader>gk"] = {
					function()
						vim.cmd("Git push")
					end,
					desc = "Git push",
				},
				["<Leader>gF"] = {
					function()
						vim.cmd("Git push --force-with-lease")
					end,
					desc = "Git push (force with lease)",
				},
				["<Leader>gK"] = {
					function()
						vim.cmd("Git pull")
					end,
					desc = "Git pull",
				},

				-- Scroll operations with centering
				["<C-e>"] = { "<C-e>zz", desc = "Scroll down 1 line" },
				["<C-y>"] = { "<C-y>zz", desc = "Scroll up 1 line" },
				["<C-d>"] = { "<C-d>zz", desc = "Scroll down 1/2 page" },
				["<C-u>"] = { "<C-u>zz", desc = "Scroll up 1/2 page" },
				["<C-f>"] = { "<C-f>zz", desc = "Scroll down 1 page" },
				["<C-b>"] = { "<C-b>zz", desc = "Scroll up 1 page" },

				-- Navigate buffer tabs with `H` and `L`
				L = {
					"<cmd>bnext<cr>",
					desc = "Next buffer",
				},
				H = {
					"<cmd>bprev<cr>",
					desc = "Previous buffer",
				},

				-- Buffer operations
				["<Leader>bD"] = {
					function()
						require("astroui.status.heirline").buffer_picker(function(bufnr)
							require("astrocore.buffer").close(bufnr)
						end)
					end,
					desc = "Pick to close",
				},
				["<Leader>b"] = { desc = "Buffers" },

				-- LSP operations
				["<F9>"] = { ":LspRestart<cr>", desc = "Restart LSPs" },

				-- TreeSJ operations
				["<Leader>m"] = {
					function()
						require("treesj").toggle()
					end,
					desc = "Toggle treesj",
				},
			},
			i = {
				-- Word deletion in insert mode
				["<C-h>"] = { "<C-w>", desc = "Delete word" },
				["<C-Del>"] = { "<C-o>dw", desc = "Delete next word" },
			},
		},
	},
}
