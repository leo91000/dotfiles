---@type LazySpec
return {
	"AstroNvim/astrocore",
	---@type AstroCoreOpts
	opts = {
		-- Configure core features of AstroNvim
		features = {
			large_buf = { size = 1024 * 500, lines = 10000 }, -- set global limits for large files for disabling features like treesitter
			autopairs = true, -- enable autopairs at start
			cmp = true, -- enable completion at start
			diagnostics_mode = 2, -- diagnostic mode on start (0 = off, 1 = no signs/virtual text, 2 = no virtual text, 3 = on)
			highlighturl = true, -- highlight URLs at start
			notifications = true, -- enable notifications at start
		},
		-- Diagnostics configuration (for vim.diagnostics.config({...})) when diagnostics are on
		diagnostics = {
			virtual_text = true,
			underline = true,
		},
		-- vim options can be configured here
		options = {
			opt = {
				relativenumber = true,
				number = true,
				spell = false,
				signcolumn = "auto",
				wrap = false,
				scrolloff = 6,
			},
			g = {},
		},
		-- Mappings can be configured through AstroCore as well.
		-- NOTE: keycodes follow the casing in the vimdocs. For example, `<Leader>` must be capitalized
		mappings = {
			-- first key is the mode
			n = {
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

				["m"] = {
					function()
						require("treesj").toggle()
					end,
					desc = "Toggle treesj",
				},

				["<leader>rn"] = {
					function()
						local is_relativenumber = vim.opt.relativenumber:get()
						vim.opt.relativenumber = not is_relativenumber
					end,
					desc = "Toggle relative line numbers",
				},
			},
			t = {
				-- setting a mapping to false will disable it
				-- ["<esc>"] = false,
			},
		},
	},
}
