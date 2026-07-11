local function is_valid_lsp_color(match)
  local color = match and match.color
  return type(color) == "table"
    and type(color.red) == "number"
    and type(color.green) == "number"
    and type(color.blue) == "number"
    and type(color.alpha) == "number"
end

return {
  "brenoprata10/nvim-highlight-colors",
  opts = function(_, opts)
    local utils = require "nvim-highlight-colors.utils"
    if utils._eliah_sanitize_lsp_document_color then return opts end

    local original = utils.highlight_lsp_document_color
    utils.highlight_lsp_document_color = function(response, ...)
      if type(response) ~= "table" then return original(response, ...) end

      local sanitized = {}
      for _, match in pairs(response) do
        if is_valid_lsp_color(match) then sanitized[#sanitized + 1] = match end
      end

      return original(sanitized, ...)
    end

    utils._eliah_sanitize_lsp_document_color = true
    return opts
  end,
}
