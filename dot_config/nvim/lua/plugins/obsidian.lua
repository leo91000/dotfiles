return {
  "epwalsh/obsidian.nvim",
  event = { "BufReadPre " .. vim.fn.expand "~/Documents/obsidian-vault" .. "/**.md" },
  dependencies = {
    "nvim-lua/plenary.nvim",
    {
      "AstroNvim/astrocore",
      opts = {
        mappings = {
          n = {
            ["gf"] = {
              function()
                if require("obsidian").util.cursor_on_markdown_link() then
                  vim.cmd.ObsidianFollowLink()
                  return
                end
                vim.cmd.normal { "gf", bang = true }
              end,
              desc = "Obsidian Follow Link",
            },
          },
        },
      },
    },
  },
  opts = function(_, opts)
    local astrocore = require "astrocore"
    return astrocore.extend_tbl(opts, {
      workspaces = {
        {
          name = "ObsidianVault",
          path = vim.env.HOME .. "/Documents/obsidian-vault",
        },
      },
      use_advanced_uri = true,
      finder = (astrocore.is_available "telescope.nvim" and "telescope.nvim")
        or (astrocore.is_available "fzf-lua" and "fzf-lua")
        or (astrocore.is_available "mini.pick" and "mini.pick"),
      templates = {
        subdir = "templates",
        date_format = "%Y-%m-%d-%a",
        time_format = "%H:%M",
      },
      completion = {
        nvim_cmp = astrocore.is_available "nvim-cmp",
      },
      note_frontmatter_func = function(note)
        local output = { id = note.id, aliases = note.aliases, tags = note.tags }
        if note.metadata ~= nil and require("obsidian").util.table_length(note.metadata) > 0 then
          for key, value in pairs(note.metadata) do
            output[key] = value
          end
        end
        return output
      end,
      follow_url_func = vim.ui.open,
    })
  end,
}
