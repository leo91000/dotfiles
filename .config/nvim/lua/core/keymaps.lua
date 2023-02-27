-- Telescope keybindings
local builtin = require('telescope.builtin')
vim.keymap.set('n', '<leader>ff', builtin.find_files, {})
vim.keymap.set('n', '<leader>fg', builtin.live_grep, {})
vim.keymap.set('n', '<leader>fb', builtin.buffers, {})
vim.keymap.set('n', '<leader>fh', builtin.help_tags, {})
vim.keymap.set('n', '<leader>fr', builtin.resume, {})
vim.keymap.set('n', '<space>gi', builtin.lsp_references, {})

local kopts = {noremap = true, silent = true}

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
vim.keymap.set('n', '<leader>gk', "<cmd>Git push --force-with-lease<cr>")
vim.keymap.set('n', 'gK', "<cmd>Git pull<cr>")
vim.keymap.set('n', 'gw', "<cmd>cnext<cr>")
vim.keymap.set('n', 'gW', "<cmd>cprevious<cr>")
vim.keymap.set('n', '<leader>gb', '<cmd>MerginalToggle<cr>')

-- Copilot
vim.keymap.set('i', '<M-)>', '<Plug>(copilot-next)')
vim.keymap.set('i', '<M-(>', '<Plug>(copilot-previous)')
vim.keymap.set('i', '<M-,>', '<Plug>(copilot-dismiss)')
vim.keymap.set('i', '<M-;>', '<Plug>(copilot-suggest)')

-- NVIM Tree
vim.keymap.set('n', '<M-&>', "<cmd>NvimTreeToggle<cr>")

-- LSP
vim.keymap.set('n', '<F9>', "<cmd>LspRestart<cr>")
