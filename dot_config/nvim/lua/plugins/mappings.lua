return {
  "AstroNvim/astrocore",
  opts = {
    mappings = {
      n = {
        ["<Leader>gk"] = { function() vim.cmd "Git push" end, desc = "Git push" },
        ["<Leader>gF"] = { function() vim.cmd "Git push --force-with-lease" end, desc = "Git push (force with lease)" },
        ["<Leader>gK"] = { function() vim.cmd "Git pull" end, desc = "Git pull" },
        ["<C-e>"] = { "<C-e>zz", desc = "Scroll down 1 line" },
        ["<C-y>"] = { "<C-y>zz", desc = "Scroll up 1 line" },
        ["<C-d>"] = { "<C-d>zz", desc = "Scroll down 1/2 page" },
        ["<C-u>"] = { "<C-u>zz", desc = "Scroll up 1/2 page" },
        ["<C-f>"] = { "<C-f>zz", desc = "Scroll down 1 page" },
        ["<C-b>"] = { "<C-b>zz", desc = "Scroll up 1 page" },
        L = { "<cmd>bnext<cr>", desc = "Next buffer" },
        H = { "<cmd>bprev<cr>", desc = "Previous buffer" },
        ["<Leader>bD"] = {
          function()
            require("astroui.status.heirline").buffer_picker(
              function(bufnr) require("astrocore.buffer").close(bufnr) end
            )
          end,
          desc = "Pick to close",
        },
        ["<Leader>b"] = { desc = "Buffers" },
        ["<F9>"] = { "<cmd>lsp restart<cr>", desc = "Restart LSPs" },
        ["<Leader>m"] = { function() require("treesj").toggle() end, desc = "Toggle treesj" },
        ["<Leader>!"] = {
          function() vim.wo.relativenumber = not vim.wo.relativenumber end,
          desc = "Toggle relative number",
        },
      },
      i = {
        ["<C-h>"] = { "<C-w>", desc = "Delete word" },
        ["<C-Del>"] = { "<C-o>dw", desc = "Delete next word" },
      },
    },
  },
}
