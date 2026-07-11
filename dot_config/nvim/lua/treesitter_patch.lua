local M = {}

local function get_parser_from_markdown_info_string(injection_alias)
  local match = vim.filetype.match { filename = "a." .. injection_alias }
  local aliases = {
    ex = "elixir",
    pl = "perl",
    sh = "bash",
    uxn = "uxntal",
    ts = "typescript",
  }
  return match or aliases[injection_alias] or injection_alias
end

local function normalize_capture_node(node)
  if node == nil then
    return nil
  end
  if type(node.range) == "function" then
    return node
  end
  if vim.islist(node) and #node > 0 then
    local last = node[#node]
    if last and type(last.range) == "function" then
      return last
    end
  end
  return nil
end

function M.apply()
  local ok = pcall(require, "nvim-treesitter.query_predicates")
  if not ok then
    return
  end
  local query = require "vim.treesitter.query"
  local opts = vim.fn.has "nvim-0.10" == 1 and { force = true, all = false } or true

  query.add_directive("set-lang-from-info-string!", function(match, _, bufnr, pred, metadata)
    local node = normalize_capture_node(match[pred[2]])
    if not node then
      return
    end
    local injection_alias = vim.treesitter.get_node_text(node, bufnr):lower()
    metadata["injection.language"] = get_parser_from_markdown_info_string(injection_alias)
  end, opts)

  query.add_directive("set-lang-from-mimetype!", function(match, _, bufnr, pred, metadata)
    local node = normalize_capture_node(match[pred[2]])
    if not node then
      return
    end
    local type_attr_value = vim.treesitter.get_node_text(node, bufnr)
    local configured = {
      ["importmap"] = "json",
      ["module"] = "javascript",
      ["application/ecmascript"] = "javascript",
      ["text/ecmascript"] = "javascript",
    }
    if configured[type_attr_value] then
      metadata["injection.language"] = configured[type_attr_value]
      return
    end
    local parts = vim.split(type_attr_value, "/", {})
    metadata["injection.language"] = parts[#parts]
  end, opts)
end

return M
