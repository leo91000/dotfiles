return {
	"supermaven-inc/supermaven-nvim",
	event = "VeryLazy",
	opts = {
		keymaps = {
			accept_suggestion = "<M-l>",
			clear_suggestion = "<M-h>",
			accept_word = "<M-w>",
		},
		log_level = "warn",
		disable_inline_completion = false, -- disables inline completion for use with cmp
		disable_keymaps = false, -- disables built in keymaps for more manual control
	},
}
