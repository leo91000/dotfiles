vim.filetype.add({ extension = { wgsl = "wgsl" } })
vim.wo.foldmethod = "expr"
vim.wo.foldexpr = "nvim_treesitter#foldexpr()"
vim.o.foldlevelstart = 99
