vim.wo.foldmethod = "expr"
vim.wo.foldexpr = "nvim_treesitter#foldexpr()"
vim.o.foldlevelstart = 99
vim.filetype.add({
	extension = {
		ejs = "ejs",
	},
})
