-- Return the argument unchanged.
function echo(r, value)
    return value
end

-- Return a stored bin.
function read_bin(r, name)
    return r[name]
end

-- Stream a stored bin of each record.
function stream_bin(stream, name)
    local function mapper(rec)
        return rec[name]
    end
    return stream : map(mapper)
end
