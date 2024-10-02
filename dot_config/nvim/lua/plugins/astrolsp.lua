-- AstroLSP allows you to customize the features in AstroNvim's LSP configuration engine
-- Configuration documentation can be found with `:h astrolsp`
-- NOTE: We highly recommend setting up the Lua Language Server (`:LspInstall lua_ls`)
--       as this provides autocomplete and documentation while editing

local function get_vue_ts_plugin_location()
	-- Get the global npm path
	local handle = io.popen("npm root -g")
	local result = handle:read("*a")
	handle:close()

	-- Trim any trailing whitespace and append the plugin path
	local global_npm_path = result:gsub("%s+", "")
	local plugin_path = global_npm_path .. "/@vue/typescript-plugin"

	-- Check if the plugin is already installed
	local plugin_exists = vim.fn.isdirectory(plugin_path) == 1

	-- If the plugin is not found, install it globally
	if not plugin_exists then
		print("@vue/typescript-plugin not found, installing globally...")
		os.execute("npm i -g @vue/typescript-plugin")
	end

	-- Return the plugin path for the language server configuration
	return plugin_path
end

local util = require("lspconfig/util")

local function has_eslint_config(root_dir)
	-- List of possible ESLint config file names
	local eslint_files = {
		".eslintrc",
		".eslintrc.js",
		".eslintrc.json",
		".eslintrc.yaml",
		".eslintrc.yml",
		"eslint.config.js",
		"eslint.config.mjs",
		"eslint.config.cjs",
	}

	-- Iterate through the list and check if any file exists in the root directory
	for _, filename in ipairs(eslint_files) do
		if util.path.exists(util.path.join(root_dir, filename)) then
			return true
		end
	end

	return false
end

---@type LazySpec
return {
	"AstroNvim/astrolsp",
	---@type AstroLSPOpts
	opts = {
		-- Configuration table of features provided by AstroLSP
		features = {
			autoformat = true, -- enable or disable auto formatting on start
			codelens = true, -- enable/disable codelens refresh on start
			inlay_hints = false, -- enable/disable inlay hints on start
			semantic_tokens = true, -- enable/disable semantic token highlighting
		},
		-- customize lsp formatting options
		formatting = {
			-- control auto formatting on save
			format_on_save = {
				enabled = true,
				allow_filetypes = { -- enable format on save for specified filetypes only
					-- "go",
				},
				-- ignore_filetypes = { -- disable format on save for specified filetypes
				-- 	-- "python",
				-- 	"vue",
				-- 	"ts",
				-- },
			},
			disabled = { -- disable formatting capabilities for the listed language servers
				"yamlls",
				"ts_ls",
				"volar",
			},
			timeout_ms = 1000, -- default format timeout
			-- filter = function(client) -- Uncomment this to debug which formatter are applied
			-- 	vim.notify("Formatting with " .. client.name)
			-- 	return true
			-- end,
		},
		-- enable servers that you already have installed without mason
		servers = {
			-- "pyright"
		},
		-- customize language server configuration options passed to `lspconfig`
		---@diagnostic disable: missing-fields
		config = {
			ts_ls = {
				init_options = {
					plugins = {
						{
							name = "@vue/typescript-plugin",
							location = get_vue_ts_plugin_location(),
							languages = { "vue", "javascript", "typescript", "typescriptreact", "javascriptreact" },
						},
					},
				},
				filetypes = { "typescript", "javascript", "typescriptreact", "javascriptreact", "vue" },
			},
			eslint = {
				---@diagnostic disable-next-line: unused-local
				on_attach = function(client, bufnr)
					local root_dir = client.config.root_dir

					if not has_eslint_config(root_dir) then
						client.stop()
						vim.notify("ESLint is disabled (no ESLint config found in project root)", vim.log.levels.WARN)
						return
					end

					-- vim.api.nvim_create_autocmd("BufWritePre", {
					-- 	buffer = bufnr,
					-- 	command = "EslintFixAll",
					-- })
				end,
			},
			rust_analyzer = {
				settings = {
					["rust-analyzer"] = {
						checkOnSave = true,
					},
				},
			},
		},
		-- customize how language servers are attached
		handlers = {
			-- a function without a key is simply the default handler, functions take two parameters, the server name and the configured options table for that server
			-- function(server, opts) require("lspconfig")[server].setup(opts) end

			-- the key is the server that is being setup with `lspconfig`
			-- rust_analyzer = false, -- setting a handler to false will disable the set up of that language server
			-- pyright = function(_, opts) require("lspconfig").pyright.setup(opts) end -- or a custom handler function can be passed
		},
		-- Configure buffer local auto commands to add when attaching a language server
		autocmds = {
			-- first key is the `augroup` to add the auto commands to (:h augroup)
			lsp_document_highlight = {
				-- Optional condition to create/delete auto command group
				-- can either be a string of a client capability or a function of `fun(client, bufnr): boolean`
				-- condition will be resolved for each client on each execution and if it ever fails for all clients,
				-- the auto commands will be deleted for that buffer
				cond = "textDocument/documentHighlight",
				-- cond = function(client, bufnr) return client.name == "lua_ls" end,
				-- list of auto commands to set
				{
					-- events to trigger
					event = { "CursorHold", "CursorHoldI" },
					-- the rest of the autocmd options (:h nvim_create_autocmd)
					desc = "Document Highlighting",
					callback = function()
						vim.lsp.buf.document_highlight()
					end,
				},
				{
					event = { "CursorMoved", "CursorMovedI", "BufLeave" },
					desc = "Document Highlighting Clear",
					callback = function()
						vim.lsp.buf.clear_references()
					end,
				},
			},
		},
		-- mappings to be set up on attaching of a language server
		mappings = {
			n = {
				gl = {
					function()
						vim.diagnostic.open_float()
					end,
					desc = "Hover diagnostics",
				},
				-- a `cond` key can provided as the string of a server capability to be required to attach, or a function with `client` and `bufnr` parameters from the `on_attach` that returns a boolean
				gD = {
					function()
						vim.lsp.buf.declaration()
					end,
					desc = "Declaration of current symbol",
					cond = "textDocument/declaration",
				},
				["<leader>dn"] = {
					function()
						vim.diagnostic.goto_next({ wrap = true })
					end,
					desc = "Go to next diagnostic",
				},
				["<leader>dp"] = {
					function()
						vim.diagnostic.goto_prev({ wrap = true })
					end,
					desc = "Go to previous diagnostic",
				},
			},
		},
		-- A custom `on_attach` function to be run after the default `on_attach` function
		-- takes two parameters `client` and `bufnr`  (`:h lspconfig-setup`)
		-- on_attach = function(client, bufnr)
		--   -- this would disable semanticTokensProvider for all clients
		--   -- client.server_capabilities.semanticTokensProvider = nil
		-- end,
	},
}
