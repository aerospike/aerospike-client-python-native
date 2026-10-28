// Copyright 2023-2026 Aerospike, Inc.
//
// Portions may be licensed to Aerospike, Inc. under one or more contributor
// license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.

use std::collections::HashMap;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};



use crate::cdt::*;
use crate::expressions::FilterExpression;
use crate::record::PythonValue;
use crate::string_ops::{StringNumericType, StringOperation};

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  OperationType (internal enum)
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Internal enum that owns all operation data, avoiding lifetime issues when converting to
    /// the core client's operation types.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum OperationType {
        /// Get operation - reads all bins from the record.
        Get(),
        /// Get operation - reads a specific bin from the record.
        GetBin(String),
        /// GetHeader operation - reads only record metadata (generation, TTL), no bin data.
        GetHeader(),
        /// Put operation - writes a bin to the record.
        Put(String, PythonValue),
        /// Add operation - increments/decrements an integer bin value.
        Add(String, PythonValue),
        /// Append operation - appends to a string bin value.
        Append(String, PythonValue),
        /// Prepend operation - prepends to a string bin value.
        Prepend(String, PythonValue),
        /// Delete operation - deletes the record.
        Delete(),
        /// Touch operation - updates the record's TTL without modifying bin data.
        Touch(),
        /// List get operation - gets element at index.
        ListGet(String, i64),
        /// List size operation - gets list size.
        ListSize(String),
        /// List join operation - concatenates the string items into one string, with an optional separator.
        ListJoin(String, Option<String>),
        /// List pop operation - pops and returns element at index.
        ListPop(String, i64),
        /// List clear operation - clears the list.
        ListClear(String),
        /// List get_range operation - gets range of elements.
        ListGetRange(String, i64, i64),
        /// List set operation - sets element at index.
        ListSet(String, i64, PythonValue),
        /// List remove operation - removes element at index.
        ListRemove(String, i64),
        /// List remove_range operation - removes range of elements.
        ListRemoveRange(String, i64, i64),
        /// List get_range_from operation - gets range from index to end.
        ListGetRangeFrom(String, i64),
        /// List pop_range operation - pops range of elements.
        ListPopRange(String, i64, i64),
        /// List pop_range_from operation - pops range from index to end.
        ListPopRangeFrom(String, i64),
        /// List remove_range_from operation - removes range from index to end.
        ListRemoveRangeFrom(String, i64),
        /// List trim operation - trims list to range.
        ListTrim(String, i64, i64),
        /// List append operation - appends a value to the list (requires ListPolicy).
        ListAppend(String, PythonValue, ListPolicy),
        /// List append_items operation - appends multiple values to the list (requires ListPolicy).
        ListAppendItems(String, Vec<PythonValue>, ListPolicy),
        /// List insert operation - inserts a value at index (requires ListPolicy).
        ListInsert(String, i64, PythonValue, ListPolicy),
        /// List insert_items operation - inserts multiple values at index (requires ListPolicy).
        ListInsertItems(String, i64, Vec<PythonValue>, ListPolicy),
        /// List increment operation - increments element at index by value (requires ListPolicy).
        ListIncrement(String, i64, i64, ListPolicy),
        /// List sort operation - sorts the list (requires ListSortFlags).
        ListSort(String, ListSortFlags),
        /// List set_order operation - sets list order (ORDERED/UNORDERED).
        ListSetOrder(String, ListOrderType),
        /// List get_by_index operation - gets element by index with return type (requires ListReturnType).
        ListGetByIndex(String, i64, ListReturnType),
        /// List get_by_index_range operation - gets elements by index range with return type (requires ListReturnType).
        ListGetByIndexRange(String, i64, Option<i64>, ListReturnType),
        /// List get_by_rank operation - gets element by rank with return type (requires ListReturnType).
        ListGetByRank(String, i64, ListReturnType),
        /// List get_by_rank_range operation - gets elements by rank range with return type (requires ListReturnType).
        ListGetByRankRange(String, i64, Option<i64>, ListReturnType),
        /// List get_by_value operation - gets elements by value with return type (requires ListReturnType).
        ListGetByValue(String, PythonValue, ListReturnType),
        /// List get_by_value_range operation - gets elements by value range with return type (requires ListReturnType).
        ListGetByValueRange(String, PythonValue, PythonValue, ListReturnType),
        /// List get_by_value_list operation - gets elements by value list with return type (requires ListReturnType).
        ListGetByValueList(String, Vec<PythonValue>, ListReturnType),
        /// List get_by_value_relative_rank_range operation - gets elements by value relative rank range (requires ListReturnType).
        ListGetByValueRelativeRankRange(String, PythonValue, i64, Option<i64>, ListReturnType),
        /// List remove_by_index operation - removes element by index with return type (requires ListReturnType).
        ListRemoveByIndex(String, i64, ListReturnType),
        /// List remove_by_index_range operation - removes elements by index range with return type (requires ListReturnType).
        ListRemoveByIndexRange(String, i64, Option<i64>, ListReturnType),
        /// List remove_by_rank operation - removes element by rank with return type (requires ListReturnType).
        ListRemoveByRank(String, i64, ListReturnType),
        /// List remove_by_rank_range operation - removes elements by rank range with return type (requires ListReturnType).
        ListRemoveByRankRange(String, i64, Option<i64>, ListReturnType),
        /// List remove_by_value operation - removes elements by value with return type (requires ListReturnType).
        ListRemoveByValue(String, PythonValue, ListReturnType),
        /// List remove_by_value_list operation - removes elements by value list with return type (requires ListReturnType).
        ListRemoveByValueList(String, Vec<PythonValue>, ListReturnType),
        /// List remove_by_value_range operation - removes elements by value range with return type (requires ListReturnType).
        ListRemoveByValueRange(String, PythonValue, PythonValue, ListReturnType),
        /// List remove_by_value_relative_rank_range operation - removes elements by value relative rank range (requires ListReturnType).
        ListRemoveByValueRelativeRankRange(String, PythonValue, i64, Option<i64>, ListReturnType),
        /// List create operation - creates a list with order and persisted index.
        ListCreate(String, ListOrderType, bool, bool),
        /// List set_with_policy operation - sets element at index with list policy.
        ListSetWithPolicy(String, i64, PythonValue, ListPolicy),
        /// List increment_by_one operation - increments element at index by 1.
        ListIncrementByOne(String, i64),
        /// List increment_by_one_with_policy operation - increments element at index by 1 with policy.
        ListIncrementByOneWithPolicy(String, i64, ListPolicy),
        /// List create_with_index operation - creates list with persisted index.
        ListCreateWithIndex(String, ListOrderType),
        /// List set_order_with_index operation - sets list order with persisted index.
        ListSetOrderWithIndex(String, ListOrderType),
        /// Map size operation - gets map size.
        MapSize(String),
        /// Map clear operation - clears the map.
        MapClear(String),
        /// Map put operation - puts a key-value pair (requires MapPolicy).
        MapPut(String, PythonValue, PythonValue, MapPolicy),
        /// Map put_items operation - puts multiple key-value pairs (requires MapPolicy).
        MapPutItems(String, Vec<(PythonValue, PythonValue)>, MapPolicy),
        /// Map increment_value operation - increments value by key (requires MapPolicy).
        MapIncrementValue(String, PythonValue, i64, MapPolicy),
        /// Map decrement_value operation - decrements value by key (requires MapPolicy).
        MapDecrementValue(String, PythonValue, i64, MapPolicy),
        /// Map get_by_key operation - gets value by key (requires MapReturnType).
        MapGetByKey(String, PythonValue, MapReturnType),
        /// Map remove_by_key operation - removes item by key (requires MapReturnType).
        MapRemoveByKey(String, PythonValue, MapReturnType),
        /// Map get_by_key_range operation - gets items by key range (requires MapReturnType).
        MapGetByKeyRange(String, PythonValue, PythonValue, MapReturnType),
        /// Map remove_by_key_range operation - removes items by key range (requires MapReturnType).
        MapRemoveByKeyRange(String, PythonValue, PythonValue, MapReturnType),
        /// Map get_by_index operation - gets item by index (requires MapReturnType).
        MapGetByIndex(String, i64, MapReturnType),
        /// Map remove_by_index operation - removes item by index (requires MapReturnType).
        MapRemoveByIndex(String, i64, MapReturnType),
        /// Map get_by_index_range operation - gets items by index range (requires MapReturnType).
        MapGetByIndexRange(String, i64, i64, MapReturnType),
        /// Map remove_by_index_range operation - removes items by index range (requires MapReturnType).
        MapRemoveByIndexRange(String, i64, i64, MapReturnType),
        /// Map get_by_index_range_from operation - gets items from index to end (requires MapReturnType).
        MapGetByIndexRangeFrom(String, i64, MapReturnType),
        /// Map remove_by_index_range_from operation - removes items from index to end (requires MapReturnType).
        MapRemoveByIndexRangeFrom(String, i64, MapReturnType),
        /// Map get_by_rank operation - gets item by rank (requires MapReturnType).
        MapGetByRank(String, i64, MapReturnType),
        /// Map remove_by_rank operation - removes item by rank (requires MapReturnType).
        MapRemoveByRank(String, i64, MapReturnType),
        /// Map get_by_rank_range operation - gets items by rank range (requires MapReturnType).
        MapGetByRankRange(String, i64, i64, MapReturnType),
        /// Map remove_by_rank_range operation - removes items by rank range (requires MapReturnType).
        MapRemoveByRankRange(String, i64, i64, MapReturnType),
        /// Map get_by_rank_range_from operation - gets items from rank to end (requires MapReturnType).
        MapGetByRankRangeFrom(String, i64, MapReturnType),
        /// Map remove_by_rank_range_from operation - removes items from rank to end (requires MapReturnType).
        MapRemoveByRankRangeFrom(String, i64, MapReturnType),
        /// Map get_by_value operation - gets items by value (requires MapReturnType).
        MapGetByValue(String, PythonValue, MapReturnType),
        /// Map remove_by_value operation - removes items by value (requires MapReturnType).
        MapRemoveByValue(String, PythonValue, MapReturnType),
        /// Map get_by_value_range operation - gets items by value range (requires MapReturnType).
        MapGetByValueRange(String, PythonValue, PythonValue, MapReturnType),
        /// Map remove_by_value_range operation - removes items by value range (requires MapReturnType).
        MapRemoveByValueRange(String, PythonValue, PythonValue, MapReturnType),
        /// Map get_by_key_list operation - gets items by a list of keys (requires MapReturnType).
        MapGetByKeyList(String, Vec<PythonValue>, MapReturnType),
        /// Map remove_by_key_list operation - removes items by a list of keys (requires MapReturnType).
        MapRemoveByKeyList(String, Vec<PythonValue>, MapReturnType),
        /// Map get_by_value_list operation - gets items by a list of values (requires MapReturnType).
        MapGetByValueList(String, Vec<PythonValue>, MapReturnType),
        /// Map remove_by_value_list operation - removes items by a list of values (requires MapReturnType).
        MapRemoveByValueList(String, Vec<PythonValue>, MapReturnType),
        /// Map set_map_policy operation - sets map policy.
        MapSetMapPolicy(String, MapPolicy),
        /// Map get_by_key_relative_index_range operation - gets items by key relative index range (requires MapReturnType).
        MapGetByKeyRelativeIndexRange(String, PythonValue, i64, Option<i64>, MapReturnType),
        /// Map get_by_value_relative_rank_range operation - gets items by value relative rank range (requires MapReturnType).
        MapGetByValueRelativeRankRange(String, PythonValue, i64, Option<i64>, MapReturnType),
        /// Map remove_by_key_relative_index_range operation - removes items by key relative index range (requires MapReturnType).
        MapRemoveByKeyRelativeIndexRange(String, PythonValue, i64, Option<i64>, MapReturnType),
        /// Map remove_by_value_relative_rank_range operation - removes items by value relative rank range (requires MapReturnType).
        MapRemoveByValueRelativeRankRange(String, PythonValue, i64, Option<i64>, MapReturnType),
        /// Map create operation - creates a map with order.
        MapCreate(String, MapOrder),
        /// Map create_with_index operation - creates map with persisted index.
        MapCreateWithIndex(String, MapOrder),
        /// Map set_policy operation - sets map policy (full policy including persist_index).
        MapSetPolicy(String, MapPolicy),
        /// Bit resize operation - resizes byte array (requires BitPolicy).
        BitResize(String, i64, Option<BitwiseResizeFlags>, BitPolicy),
        /// Bit insert operation - inserts bytes (requires BitPolicy).
        BitInsert(String, i64, PythonValue, BitPolicy),
        /// Bit remove operation - removes bytes (requires BitPolicy).
        BitRemove(String, i64, i64, BitPolicy),
        /// Bit set operation - sets bits (requires BitPolicy).
        BitSet(String, i64, i64, PythonValue, BitPolicy),
        /// Bit or operation - performs bitwise OR (requires BitPolicy).
        BitOr(String, i64, i64, PythonValue, BitPolicy),
        /// Bit xor operation - performs bitwise XOR (requires BitPolicy).
        BitXor(String, i64, i64, PythonValue, BitPolicy),
        /// Bit and operation - performs bitwise AND (requires BitPolicy).
        BitAnd(String, i64, i64, PythonValue, BitPolicy),
        /// Bit not operation - performs bitwise NOT (requires BitPolicy).
        BitNot(String, i64, i64, BitPolicy),
        /// Bit lshift operation - performs left shift (requires BitPolicy).
        BitLShift(String, i64, i64, i64, BitPolicy),
        /// Bit rshift operation - performs right shift (requires BitPolicy).
        BitRShift(String, i64, i64, i64, BitPolicy),
        /// Bit add operation - adds to integer value (requires BitPolicy).
        BitAdd(String, i64, i64, i64, bool, BitwiseOverflowActions, BitPolicy),
        /// Bit subtract operation - subtracts from integer value (requires BitPolicy).
        BitSubtract(String, i64, i64, i64, bool, BitwiseOverflowActions, BitPolicy),
        /// Bit set_int operation - sets integer value (requires BitPolicy).
        BitSetInt(String, i64, i64, i64, BitPolicy),
        /// Bit get operation - gets bits (read-only).
        BitGet(String, i64, i64),
        /// Bit count operation - counts set bits (read-only).
        BitCount(String, i64, i64),
        /// Bit lscan operation - scans left for value (read-only).
        BitLScan(String, i64, i64, bool),
        /// Bit rscan operation - scans right for value (read-only).
        BitRScan(String, i64, i64, bool),
        /// Bit get_int operation - gets integer value (read-only).
        BitGetInt(String, i64, i64, bool),
        /// Bit b64_encode operation - returns base64 text of the blob (read-only).
        BitB64Encode(String, Option<i64>, Option<i64>, bool),
        /// HLL init operation - creates or resets an HLL bin.
        HllInit(String, i64, i64, u8),
        /// HLL add operation - adds values to HLL.
        HllAdd(String, Vec<PythonValue>, i64, i64, u8),
        /// HLL get_count operation - returns estimated count.
        HllGetCount(String),
        /// HLL describe operation - returns index_bit_count and min_hash_bit_count.
        HllDescribe(String),
        /// HLL refresh_count operation - updates cached count.
        HllRefreshCount(String),
        /// HLL fold operation - folds HLL to specified index_bit_count.
        HllFold(String, i64),
        /// HLL get_union operation - returns union of HLL objects.
        HllGetUnion(String, Vec<PythonValue>),
        /// HLL get_union_count operation - returns estimated union count.
        HllGetUnionCount(String, Vec<PythonValue>),
        /// HLL get_intersect_count operation - returns estimated intersection count.
        HllGetIntersectCount(String, Vec<PythonValue>),
        /// HLL get_similarity operation - returns estimated similarity.
        HllGetSimilarity(String, Vec<PythonValue>),
        /// HLL set_union operation - sets union of HLL objects.
        HllSetUnion(String, Vec<PythonValue>, u8),
        /// Expression read operation - evaluates expression and returns result.
        ExpRead(String, FilterExpression, i64),
        /// Expression write operation - evaluates expression and writes result to bin.
        ExpWrite(String, FilterExpression, i64),
        /// CDT path select operation — reads nested CDT data by path expression.
        CdtSelectByPath(String, i64, Vec<CTX>),
        /// CDT path modify operation — writes nested CDT data by path expression.
        CdtModifyByPath(String, i64, FilterExpression, Vec<CTX>),

        // ----- String operations (server 8.2.0+) -----
        // STRING_READ — sub-ops 0..16. No write_flags field.
        /// String strlen — codepoint count.
        StringStrlen(String),
        /// String substr (bin, start, end). end=None means substr-from-start to end.
        StringSubstr(String, i64, Option<i64>),
        /// String char_at (bin, index).
        StringCharAt(String, i64),
        /// String find (bin, needle, occurrence). occurrence=None means first match.
        StringFind(String, String, Option<i64>),
        /// String contains (bin, needle) — boolean return.
        StringContains(String, String),
        /// String starts_with (bin, prefix) — boolean return.
        StringStartsWith(String, String),
        /// String ends_with (bin, suffix) — boolean return.
        StringEndsWith(String, String),
        /// String to_integer.
        StringToInteger(String),
        /// String to_double.
        StringToDouble(String),
        /// String byte_length — UTF-8 byte length.
        StringByteLength(String),
        /// String is_numeric (bin, numeric_type). numeric_type=None means ANY.
        StringIsNumeric(String, Option<StringNumericType>),
        /// String is_upper — boolean return.
        StringIsUpper(String),
        /// String is_lower — boolean return.
        StringIsLower(String),
        /// String to_blob — UTF-8 bytes as blob.
        StringToBlob(String),
        /// String split (bin, separator). separator=None means codepoint-by-codepoint.
        StringSplit(String, Option<String>),
        /// String b64_decode.
        StringB64Decode(String),
        /// String regex_compare (bin, pattern, regex_flags) — boolean return.
        StringRegexCompare(String, String, u8),

        // STRING_MODIFY — sub-ops 50..66. Every variant carries write_flags as last u8;
        // StringRegexReplace carries regex_flags before it.
        /// String insert (bin, index, value, write_flags).
        StringInsert(String, i64, String, u8),
        /// String overwrite (bin, index, value, write_flags).
        StringOverwrite(String, i64, String, u8),
        /// String concat (bin, values, write_flags) — wire is always list-of-strings.
        StringConcat(String, Vec<String>, u8),
        /// String append (bin, value, write_flags) — single-value form, sub-op 67.
        StringAppend(String, String, u8),
        /// String prepend (bin, value, write_flags) — single-value form, sub-op 68.
        StringPrepend(String, String, u8),
        /// String snip (bin, start, end, write_flags). end=None means snip from
        /// start through the end of the string; that form packs `[53, start]`
        /// with no flags element — the server reads snip args by position, so a
        /// 2-element `[start, flags]` wire would land the flags in the `end`
        /// slot and silently snip the empty range `[start, 0)`.
        StringSnip(String, i64, Option<i64>, u8),
        /// String replace (bin, needle, replacement, write_flags) — first match only.
        StringReplace(String, String, String, u8),
        /// String replace_all (bin, needle, replacement, write_flags) — all matches.
        StringReplaceAll(String, String, String, u8),
        /// String upper (bin, write_flags).
        StringUpper(String, u8),
        /// String lower (bin, write_flags).
        StringLower(String, u8),
        /// String case_fold (bin, write_flags) — locale-independent lowercase.
        StringCaseFold(String, u8),
        /// String normalize_nfc (bin, write_flags) — Unicode NFC normalize.
        StringNormalizeNfc(String, u8),
        /// String trim_start (bin, write_flags).
        StringTrimStart(String, u8),
        /// String trim_end (bin, write_flags).
        StringTrimEnd(String, u8),
        /// String trim (bin, write_flags) — both ends.
        StringTrim(String, u8),
        /// String pad_start (bin, target_length, pad_string, write_flags).
        StringPadStart(String, i64, String, u8),
        /// String pad_end (bin, target_length, pad_string, write_flags).
        StringPadEnd(String, i64, String, u8),
        /// String repeat (bin, count, write_flags).
        StringRepeat(String, i64, u8),
        /// String regex_replace (bin, pattern, replacement, regex_flags, write_flags).
        /// Two flag words: the regex flags select ICU behavior, the write flags fill
        /// the same trailing policy slot every other modify op carries.
        StringRegexReplace(String, String, String, u8, u8),

        // TO_STRING — op-type 19. Top-level wire op, no payload, no sub-op id, no CTX.
        /// String to_string (bin) — convert non-string bin to string repr.
        StringToString(String),
    }

    impl OperationType {
        /// Whether re-sending this operation after an in-doubt attempt leaves
        /// the record as one application would.
        ///
        /// Reads, `Put`, `Delete`, and `Touch` are idempotent. `Add`, `Append`,
        /// `Prepend`, and every list, map, bit, HLL, string, and expression
        /// modify operation are reported non-idempotent as a family: a few
        /// members are idempotent in effect (`ListClear`, `HllAdd`), but the
        /// family is what retry classification keys on, and refining a member
        /// is a change to this table alone. The match is exhaustive on
        /// purpose, so a new variant has to be placed here before it builds.
        pub(crate) fn is_idempotent(&self) -> bool {
            match self {
                OperationType::Get()
                | OperationType::GetBin(_)
                | OperationType::GetHeader()
                | OperationType::Put(..)
                | OperationType::Delete()
                | OperationType::Touch()
                | OperationType::ListGet(..)
                | OperationType::ListSize(_)
                | OperationType::ListJoin(..)
                | OperationType::ListGetRange(..)
                | OperationType::ListGetRangeFrom(..)
                | OperationType::ListGetByIndex(..)
                | OperationType::ListGetByIndexRange(..)
                | OperationType::ListGetByRank(..)
                | OperationType::ListGetByRankRange(..)
                | OperationType::ListGetByValue(..)
                | OperationType::ListGetByValueRange(..)
                | OperationType::ListGetByValueList(..)
                | OperationType::ListGetByValueRelativeRankRange(..)
                | OperationType::MapSize(_)
                | OperationType::MapGetByKey(..)
                | OperationType::MapGetByKeyRange(..)
                | OperationType::MapGetByIndex(..)
                | OperationType::MapGetByIndexRange(..)
                | OperationType::MapGetByIndexRangeFrom(..)
                | OperationType::MapGetByRank(..)
                | OperationType::MapGetByRankRange(..)
                | OperationType::MapGetByRankRangeFrom(..)
                | OperationType::MapGetByValue(..)
                | OperationType::MapGetByValueRange(..)
                | OperationType::MapGetByKeyList(..)
                | OperationType::MapGetByValueList(..)
                | OperationType::MapGetByKeyRelativeIndexRange(..)
                | OperationType::MapGetByValueRelativeRankRange(..)
                | OperationType::BitGet(..)
                | OperationType::BitCount(..)
                | OperationType::BitLScan(..)
                | OperationType::BitRScan(..)
                | OperationType::BitGetInt(..)
                | OperationType::BitB64Encode(..)
                | OperationType::HllGetCount(_)
                | OperationType::HllDescribe(_)
                | OperationType::HllGetUnion(..)
                | OperationType::HllGetUnionCount(..)
                | OperationType::HllGetIntersectCount(..)
                | OperationType::HllGetSimilarity(..)
                | OperationType::ExpRead(..)
                | OperationType::CdtSelectByPath(..)
                | OperationType::StringStrlen(_)
                | OperationType::StringSubstr(..)
                | OperationType::StringCharAt(..)
                | OperationType::StringFind(..)
                | OperationType::StringContains(..)
                | OperationType::StringStartsWith(..)
                | OperationType::StringEndsWith(..)
                | OperationType::StringToInteger(_)
                | OperationType::StringToDouble(_)
                | OperationType::StringByteLength(_)
                | OperationType::StringIsNumeric(..)
                | OperationType::StringIsUpper(_)
                | OperationType::StringIsLower(_)
                | OperationType::StringToBlob(_)
                | OperationType::StringSplit(..)
                | OperationType::StringB64Decode(_)
                | OperationType::StringRegexCompare(..)
                | OperationType::StringToString(_) => true,
                OperationType::Add(..)
                | OperationType::Append(..)
                | OperationType::Prepend(..)
                | OperationType::ListPop(..)
                | OperationType::ListClear(_)
                | OperationType::ListSet(..)
                | OperationType::ListRemove(..)
                | OperationType::ListRemoveRange(..)
                | OperationType::ListPopRange(..)
                | OperationType::ListPopRangeFrom(..)
                | OperationType::ListRemoveRangeFrom(..)
                | OperationType::ListTrim(..)
                | OperationType::ListAppend(..)
                | OperationType::ListAppendItems(..)
                | OperationType::ListInsert(..)
                | OperationType::ListInsertItems(..)
                | OperationType::ListIncrement(..)
                | OperationType::ListSort(..)
                | OperationType::ListSetOrder(..)
                | OperationType::ListRemoveByIndex(..)
                | OperationType::ListRemoveByIndexRange(..)
                | OperationType::ListRemoveByRank(..)
                | OperationType::ListRemoveByRankRange(..)
                | OperationType::ListRemoveByValue(..)
                | OperationType::ListRemoveByValueList(..)
                | OperationType::ListRemoveByValueRange(..)
                | OperationType::ListRemoveByValueRelativeRankRange(..)
                | OperationType::ListCreate(..)
                | OperationType::ListSetWithPolicy(..)
                | OperationType::ListIncrementByOne(..)
                | OperationType::ListIncrementByOneWithPolicy(..)
                | OperationType::ListCreateWithIndex(..)
                | OperationType::ListSetOrderWithIndex(..)
                | OperationType::MapClear(_)
                | OperationType::MapPut(..)
                | OperationType::MapPutItems(..)
                | OperationType::MapIncrementValue(..)
                | OperationType::MapDecrementValue(..)
                | OperationType::MapRemoveByKey(..)
                | OperationType::MapRemoveByKeyRange(..)
                | OperationType::MapRemoveByIndex(..)
                | OperationType::MapRemoveByIndexRange(..)
                | OperationType::MapRemoveByIndexRangeFrom(..)
                | OperationType::MapRemoveByRank(..)
                | OperationType::MapRemoveByRankRange(..)
                | OperationType::MapRemoveByRankRangeFrom(..)
                | OperationType::MapRemoveByValue(..)
                | OperationType::MapRemoveByValueRange(..)
                | OperationType::MapRemoveByKeyList(..)
                | OperationType::MapRemoveByValueList(..)
                | OperationType::MapSetMapPolicy(..)
                | OperationType::MapRemoveByKeyRelativeIndexRange(..)
                | OperationType::MapRemoveByValueRelativeRankRange(..)
                | OperationType::MapCreate(..)
                | OperationType::MapCreateWithIndex(..)
                | OperationType::MapSetPolicy(..)
                | OperationType::BitResize(..)
                | OperationType::BitInsert(..)
                | OperationType::BitRemove(..)
                | OperationType::BitSet(..)
                | OperationType::BitOr(..)
                | OperationType::BitXor(..)
                | OperationType::BitAnd(..)
                | OperationType::BitNot(..)
                | OperationType::BitLShift(..)
                | OperationType::BitRShift(..)
                | OperationType::BitAdd(..)
                | OperationType::BitSubtract(..)
                | OperationType::BitSetInt(..)
                | OperationType::HllInit(..)
                | OperationType::HllAdd(..)
                | OperationType::HllRefreshCount(_)
                | OperationType::HllFold(..)
                | OperationType::HllSetUnion(..)
                | OperationType::ExpWrite(..)
                | OperationType::CdtModifyByPath(..)
                | OperationType::StringInsert(..)
                | OperationType::StringOverwrite(..)
                | OperationType::StringConcat(..)
                | OperationType::StringAppend(..)
                | OperationType::StringPrepend(..)
                | OperationType::StringSnip(..)
                | OperationType::StringReplace(..)
                | OperationType::StringReplaceAll(..)
                | OperationType::StringUpper(..)
                | OperationType::StringLower(..)
                | OperationType::StringCaseFold(..)
                | OperationType::StringNormalizeNfc(..)
                | OperationType::StringTrimStart(..)
                | OperationType::StringTrimEnd(..)
                | OperationType::StringTrim(..)
                | OperationType::StringPadStart(..)
                | OperationType::StringPadEnd(..)
                | OperationType::StringRepeat(..)
                | OperationType::StringRegexReplace(..) => false,
            }
        }
    }

    /// Python wrapper for Operation enum.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct Operation {
        pub(crate) op: OperationType,
    }

    impl Default for Operation {
        fn default() -> Self {
            Operation {
                op: OperationType::Get(),
            }
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Operation {
        #[new]
        pub fn new() -> Self {
            Self::default()
        }

        /// ``True`` when re-sending this operation after an in-doubt attempt
        /// leaves the record as one application would, so a retry is safe.
        ///
        /// Reads, ``put``, ``delete``, and ``touch`` are idempotent. ``add``,
        /// ``append``, ``prepend``, and every list, map, bit, HLL, string, and
        /// expression modify operation are not, as a family: a few members are
        /// idempotent in effect (list ``clear``, HLL ``add``), but retry
        /// classification keys on the family. Every operation class carries
        /// this property.
        #[getter]
        pub fn is_idempotent(&self) -> bool {
            self.op.is_idempotent()
        }

        /// Name of the bin this operation targets, or ``None`` when it targets
        /// the whole record.
        ///
        /// Lets a caller report *which* bin an operation names when the server
        /// rejects one — the server reports a bad bin name without saying which
        /// bin it was, and the name is otherwise consumed at construction.
        ///
        /// Covers the operations this class constructs. Bin, list, map, HLL, and
        /// expression operations are separate classes and answer ``None`` here.
        #[getter]
        pub fn bin_name(&self) -> Option<&str> {
            match &self.op {
                OperationType::GetBin(name)
                | OperationType::Put(name, _)
                | OperationType::Add(name, _)
                | OperationType::Append(name, _)
                | OperationType::Prepend(name, _) => Some(name),
                // Get / GetHeader / Delete / Touch address the record, not a bin.
                _ => None,
            }
        }

        /// Create a Get operation (reads all bins).
        #[staticmethod]
        pub fn get() -> Self {
            Operation {
                op: OperationType::Get(),
            }
        }

        /// Create a Get operation for a specific bin.
        #[staticmethod]
        pub fn get_bin(bin_name: String) -> Self {
            Operation {
                op: OperationType::GetBin(bin_name),
            }
        }

        /// Create a Put operation.
        #[staticmethod]
        pub fn put(bin_name: String, value: PythonValue) -> Self {
            Operation {
                op: OperationType::Put(bin_name, value),
            }
        }

        /// Create a GetHeader operation (metadata only, no bin data).
        #[staticmethod]
        pub fn get_header() -> Self {
            Operation {
                op: OperationType::GetHeader(),
            }
        }

        /// Create a Delete operation.
        #[staticmethod]
        pub fn delete() -> Self {
            Operation {
                op: OperationType::Delete(),
            }
        }

        /// Create a Touch operation (updates TTL).
        #[staticmethod]
        pub fn touch() -> Self {
            Operation {
                op: OperationType::Touch(),
            }
        }

        /// Create an Add operation (increments/decrements integer bin value).
        #[staticmethod]
        pub fn add(bin_name: String, value: PythonValue) -> Self {
            Operation {
                op: OperationType::Add(bin_name, value),
            }
        }

        /// Create an Append operation (byte-level append to a string or bytes bin).
        #[staticmethod]
        pub fn append(bin_name: String, value: PythonValue) -> Self {
            Operation {
                op: OperationType::Append(bin_name, value),
            }
        }

        /// Create a Prepend operation (byte-level prepend to a string or bytes bin).
        #[staticmethod]
        pub fn prepend(bin_name: String, value: PythonValue) -> Self {
            Operation {
                op: OperationType::Prepend(bin_name, value),
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ListOperation
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// List bin operations. Create list operations used by the client's `operate()` method.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct ListOperation {
        pub(crate) op: OperationType,
        pub(crate) ctx: Option<Vec<CTX>>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl ListOperation {
        /// ``True`` when re-sending this operation after an in-doubt attempt
        /// leaves the record as one application would. Reads are idempotent;
        /// every modify operation of this class is not. See
        /// :attr:`Operation.is_idempotent` for the rule.
        #[getter]
        pub fn is_idempotent(&self) -> bool {
            self.op.is_idempotent()
        }

        /// Create a List get operation (gets element at index).
        #[staticmethod]
        pub fn get(bin_name: String, index: i64) -> Self {
            ListOperation {
                op: OperationType::ListGet(bin_name, index),
                ctx: None,
            }
        }

        /// Create a List size operation (gets list size).
        #[staticmethod]
        pub fn size(bin_name: String) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListSize(bin_name),
            }
        }

        /// Create a List join operation (concatenates the string items into
        /// one string). With ``separator`` set, it is inserted between
        /// consecutive items; an empty list joins to an empty string. The
        /// list must hold only strings — anything else fails with
        /// ``PARAMETER_ERROR``. The inverse of ``StringOperation.split``.
        /// Requires server 8.2.0+.
        #[staticmethod]
        #[pyo3(signature = (bin_name, separator=None))]
        pub fn join(bin_name: String, separator: Option<String>) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListJoin(bin_name, separator),
            }
        }

        /// Create a List pop operation (pops and returns element at index).
        #[staticmethod]
        pub fn pop(bin_name: String, index: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListPop(bin_name, index),
            }
        }

        /// Create a List clear operation (clears the list).
        #[staticmethod]
        pub fn clear(bin_name: String) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListClear(bin_name),
            }
        }

        /// Create a List get_range operation (gets range of elements).
        #[staticmethod]
        pub fn get_range(bin_name: String, index: i64, count: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetRange(bin_name, index, count),
            }
        }

        /// Create a List set operation (sets element at index).
        #[staticmethod]
        pub fn set(bin_name: String, index: i64, value: PythonValue) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListSet(bin_name, index, value),
            }
        }

        /// Create a List set_with_policy operation (sets element at index with list policy).
        #[staticmethod]
        pub fn set_with_policy(bin_name: String, policy: ListPolicy, index: i64, value: PythonValue) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListSetWithPolicy(bin_name, index, value, policy),
            }
        }

        /// Create a List remove operation (removes element at index).
        #[staticmethod]
        pub fn remove(bin_name: String, index: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemove(bin_name, index),
            }
        }

        /// Create a List remove_range operation (removes range of elements).
        #[staticmethod]
        pub fn remove_range(bin_name: String, index: i64, count: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveRange(bin_name, index, count),
            }
        }

        /// Create a List get_range_from operation (gets range from index to end).
        #[staticmethod]
        pub fn get_range_from(bin_name: String, index: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetRangeFrom(bin_name, index),
            }
        }

        /// Create a List pop_range operation (pops range of elements).
        #[staticmethod]
        pub fn pop_range(bin_name: String, index: i64, count: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListPopRange(bin_name, index, count),
            }
        }

        /// Create a List pop_range_from operation (pops range from index to end).
        #[staticmethod]
        pub fn pop_range_from(bin_name: String, index: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListPopRangeFrom(bin_name, index),
            }
        }

        /// Create a List remove_range_from operation (removes range from index to end).
        #[staticmethod]
        pub fn remove_range_from(bin_name: String, index: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveRangeFrom(bin_name, index),
            }
        }

        /// Create a List trim operation (trims list to range).
        #[staticmethod]
        pub fn trim(bin_name: String, index: i64, count: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListTrim(bin_name, index, count),
            }
        }

        /// Create a List append operation (appends a value to the list, requires ListPolicy).
        #[staticmethod]
        pub fn append(bin_name: String, value: PythonValue, policy: ListPolicy) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListAppend(bin_name, value, policy),
            }
        }

        /// Create a List append_items operation (appends multiple values to the list, requires ListPolicy).
        #[staticmethod]
        pub fn append_items(bin_name: String, values: Vec<PythonValue>, policy: ListPolicy) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListAppendItems(bin_name, values, policy),
            }
        }

        /// Create a List insert operation (inserts a value at index, requires ListPolicy).
        #[staticmethod]
        pub fn insert(bin_name: String, index: i64, value: PythonValue, policy: ListPolicy) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListInsert(bin_name, index, value, policy),
            }
        }

        /// Create a List insert_items operation (inserts multiple values at index, requires ListPolicy).
        #[staticmethod]
        pub fn insert_items(bin_name: String, index: i64, values: Vec<PythonValue>, policy: ListPolicy) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListInsertItems(bin_name, index, values, policy),
            }
        }

        /// Create a List increment operation (increments element at index by value, requires ListPolicy).
        #[staticmethod]
        pub fn increment(bin_name: String, index: i64, value: i64, policy: ListPolicy) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListIncrement(bin_name, index, value, policy),
            }
        }

        /// Create a List increment_by_one operation (increments element at index by 1).
        #[staticmethod]
        pub fn increment_by_one(bin_name: String, index: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListIncrementByOne(bin_name, index),
            }
        }

        /// Create a List increment_by_one_with_policy operation (increments element at index by 1 with policy).
        #[staticmethod]
        pub fn increment_by_one_with_policy(bin_name: String, policy: ListPolicy, index: i64) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListIncrementByOneWithPolicy(bin_name, index, policy),
            }
        }

        /// Create a List sort operation (sorts the list, requires ListSortFlags).
        #[staticmethod]
        pub fn sort(bin_name: String, flags: ListSortFlags) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListSort(bin_name, flags),
            }
        }

        /// Create a List set_order operation (sets list order ORDERED/UNORDERED).
        #[staticmethod]
        pub fn set_order(bin_name: String, order: ListOrderType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListSetOrder(bin_name, order),
            }
        }

        /// Create a List get_by_index operation (gets element by index with return type, requires ListReturnType).
        #[staticmethod]
        pub fn get_by_index(bin_name: String, index: i64, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByIndex(bin_name, index, return_type),
            }
        }

        /// Create a List get_by_index_range operation (gets elements by index range with return type, requires ListReturnType).
        /// If count is None, gets from index to end of list.
        #[staticmethod]
        pub fn get_by_index_range(bin_name: String, index: i64, count: Option<i64>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByIndexRange(bin_name, index, count, return_type),
            }
        }

        /// Create a List get_by_rank operation (gets element by rank with return type, requires ListReturnType).
        #[staticmethod]
        pub fn get_by_rank(bin_name: String, rank: i64, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByRank(bin_name, rank, return_type),
            }
        }

        /// Create a List get_by_rank_range operation (gets elements by rank range with return type, requires ListReturnType).
        /// If count is None, gets from rank to end of list.
        #[staticmethod]
        pub fn get_by_rank_range(bin_name: String, rank: i64, count: Option<i64>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByRankRange(bin_name, rank, count, return_type),
            }
        }

        /// Create a List get_by_value operation (gets elements by value with return type, requires ListReturnType).
        #[staticmethod]
        pub fn get_by_value(bin_name: String, value: PythonValue, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByValue(bin_name, value, return_type),
            }
        }

        /// Create a List get_by_value_range operation (gets elements by value range with return type, requires ListReturnType).
        #[staticmethod]
        pub fn get_by_value_range(bin_name: String, begin: PythonValue, end: PythonValue, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByValueRange(bin_name, begin, end, return_type),
            }
        }

        /// Create a List get_by_value_list operation (gets elements by value list with return type, requires ListReturnType).
        #[staticmethod]
        pub fn get_by_value_list(bin_name: String, values: Vec<PythonValue>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByValueList(bin_name, values, return_type),
            }
        }

        /// Create a List get_by_value_relative_rank_range operation (gets elements by value relative rank range, requires ListReturnType).
        #[staticmethod]
        pub fn get_by_value_relative_rank_range(bin_name: String, value: PythonValue, rank: i64, count: Option<i64>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListGetByValueRelativeRankRange(bin_name, value, rank, count, return_type),
            }
        }

        /// Create a List remove_by_index operation (removes element by index with return type, requires ListReturnType).
        #[staticmethod]
        pub fn remove_by_index(bin_name: String, index: i64, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByIndex(bin_name, index, return_type),
            }
        }

        /// Create a List remove_by_index_range operation (removes elements by index range with return type, requires ListReturnType).
        /// If count is None, removes from index to end of list.
        #[staticmethod]
        pub fn remove_by_index_range(bin_name: String, index: i64, count: Option<i64>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByIndexRange(bin_name, index, count, return_type),
            }
        }

        /// Create a List remove_by_rank operation (removes element by rank with return type, requires ListReturnType).
        #[staticmethod]
        pub fn remove_by_rank(bin_name: String, rank: i64, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByRank(bin_name, rank, return_type),
            }
        }

        /// Create a List remove_by_rank_range operation (removes elements by rank range with return type, requires ListReturnType).
        /// If count is None, removes from rank to end of list.
        #[staticmethod]
        pub fn remove_by_rank_range(bin_name: String, rank: i64, count: Option<i64>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByRankRange(bin_name, rank, count, return_type),
            }
        }

        /// Create a List remove_by_value operation (removes elements by value with return type, requires ListReturnType).
        #[staticmethod]
        pub fn remove_by_value(bin_name: String, value: PythonValue, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByValue(bin_name, value, return_type),
            }
        }

        /// Create a List remove_by_value_list operation (removes elements by value list with return type, requires ListReturnType).
        #[staticmethod]
        pub fn remove_by_value_list(bin_name: String, values: Vec<PythonValue>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByValueList(bin_name, values, return_type),
            }
        }

        /// Create a List remove_by_value_range operation (removes elements by value range with return type, requires ListReturnType).
        #[staticmethod]
        pub fn remove_by_value_range(bin_name: String, begin: PythonValue, end: PythonValue, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByValueRange(bin_name, begin, end, return_type),
            }
        }

        /// Create a List remove_by_value_relative_rank_range operation (removes elements by value relative rank range, requires ListReturnType).
        #[staticmethod]
        pub fn remove_by_value_relative_rank_range(bin_name: String, value: PythonValue, rank: i64, count: Option<i64>, return_type: ListReturnType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListRemoveByValueRelativeRankRange(bin_name, value, rank, count, return_type),
            }
        }

        /// Create a List create operation (creates a list with order and persisted index).
        #[staticmethod]
        pub fn create(bin_name: String, order: ListOrderType, pad: bool, persist_index: bool) -> Self {
            ListOperation {
                op: OperationType::ListCreate(bin_name, order, pad, persist_index),
                ctx: None,
            }
        }

        /// Create a List create_with_index operation (creates list with persisted index).
        #[staticmethod]
        pub fn create_with_index(bin_name: String, order: ListOrderType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListCreateWithIndex(bin_name, order),
            }
        }

        /// Create a List set_order_with_index operation (sets list order with persisted index).
        #[staticmethod]
        pub fn set_order_with_index(bin_name: String, order: ListOrderType) -> Self {
            ListOperation {
                ctx: None,
                op: OperationType::ListSetOrderWithIndex(bin_name, order),
            }
        }

        /// Set the context for this operation. Used for nested CDT operations.
        pub fn set_context(&self, ctx: Vec<CTX>) -> Self {
            ListOperation {
                op: self.op.clone(),
                ctx: Some(ctx),
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  MapOperation
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Map bin operations. Create map operations used by the client's `operate()` method.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct MapOperation {
        pub(crate) op: OperationType,
        pub(crate) ctx: Option<Vec<CTX>>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl MapOperation {
        /// ``True`` when re-sending this operation after an in-doubt attempt
        /// leaves the record as one application would. Reads are idempotent;
        /// every modify operation of this class is not. See
        /// :attr:`Operation.is_idempotent` for the rule.
        #[getter]
        pub fn is_idempotent(&self) -> bool {
            self.op.is_idempotent()
        }

        /// Create a Map size operation (gets map size).
        #[staticmethod]
        pub fn size(bin_name: String) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapSize(bin_name),
            }
        }

        #[staticmethod]
        pub fn clear(bin_name: String) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapClear(bin_name),
            }
        }

        #[staticmethod]
        pub fn put(bin_name: String, key: PythonValue, value: PythonValue, policy: MapPolicy) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapPut(bin_name, key, value, policy),
            }
        }

        #[staticmethod]
        pub fn put_items(bin_name: String, items: Vec<(PythonValue, PythonValue)>, policy: MapPolicy) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapPutItems(bin_name, items, policy),
            }
        }

        #[staticmethod]
        pub fn increment_value(bin_name: String, key: PythonValue, value: i64, policy: MapPolicy) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapIncrementValue(bin_name, key, value, policy),
            }
        }

        #[staticmethod]
        pub fn decrement_value(bin_name: String, key: PythonValue, value: i64, policy: MapPolicy) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapDecrementValue(bin_name, key, value, policy),
            }
        }

        #[staticmethod]
        pub fn get_by_key(bin_name: String, key: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByKey(bin_name, key, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_key(bin_name: String, key: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByKey(bin_name, key, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_key_range(bin_name: String, begin: PythonValue, end: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByKeyRange(bin_name, begin, end, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_key_range(bin_name: String, begin: PythonValue, end: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByKeyRange(bin_name, begin, end, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_index(bin_name: String, index: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByIndex(bin_name, index, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_index(bin_name: String, index: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByIndex(bin_name, index, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_index_range(bin_name: String, index: i64, count: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByIndexRange(bin_name, index, count, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_index_range(bin_name: String, index: i64, count: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByIndexRange(bin_name, index, count, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_index_range_from(bin_name: String, index: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByIndexRangeFrom(bin_name, index, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_index_range_from(bin_name: String, index: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByIndexRangeFrom(bin_name, index, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_rank(bin_name: String, rank: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByRank(bin_name, rank, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_rank(bin_name: String, rank: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByRank(bin_name, rank, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_rank_range(bin_name: String, rank: i64, count: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByRankRange(bin_name, rank, count, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_rank_range(bin_name: String, rank: i64, count: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByRankRange(bin_name, rank, count, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_rank_range_from(bin_name: String, rank: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByRankRangeFrom(bin_name, rank, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_rank_range_from(bin_name: String, rank: i64, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByRankRangeFrom(bin_name, rank, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_value(bin_name: String, value: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByValue(bin_name, value, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_value(bin_name: String, value: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByValue(bin_name, value, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_value_range(bin_name: String, begin: PythonValue, end: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByValueRange(bin_name, begin, end, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_value_range(bin_name: String, begin: PythonValue, end: PythonValue, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByValueRange(bin_name, begin, end, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_key_list(bin_name: String, keys: Vec<PythonValue>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByKeyList(bin_name, keys, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_key_list(bin_name: String, keys: Vec<PythonValue>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByKeyList(bin_name, keys, return_type),
            }
        }

        #[staticmethod]
        pub fn get_by_value_list(bin_name: String, values: Vec<PythonValue>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByValueList(bin_name, values, return_type),
            }
        }

        #[staticmethod]
        pub fn remove_by_value_list(bin_name: String, values: Vec<PythonValue>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByValueList(bin_name, values, return_type),
            }
        }

        /// Create a Map set_map_policy operation (sets map policy).
        #[staticmethod]
        pub fn set_map_policy(bin_name: String, policy: MapPolicy) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapSetMapPolicy(bin_name, policy),
            }
        }

        /// Create a Map get_by_key_relative_index_range operation (gets items by key relative index range, requires MapReturnType).
        #[staticmethod]
        pub fn get_by_key_relative_index_range(bin_name: String, key: PythonValue, index: i64, count: Option<i64>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByKeyRelativeIndexRange(bin_name, key, index, count, return_type),
            }
        }

        /// Create a Map get_by_value_relative_rank_range operation (gets items by value relative rank range, requires MapReturnType).
        #[staticmethod]
        pub fn get_by_value_relative_rank_range(bin_name: String, value: PythonValue, rank: i64, count: Option<i64>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapGetByValueRelativeRankRange(bin_name, value, rank, count, return_type),
            }
        }

        /// Create a Map remove_by_key_relative_index_range operation (removes items by key relative index range, requires MapReturnType).
        #[staticmethod]
        pub fn remove_by_key_relative_index_range(bin_name: String, key: PythonValue, index: i64, count: Option<i64>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByKeyRelativeIndexRange(bin_name, key, index, count, return_type),
            }
        }

        /// Create a Map remove_by_value_relative_rank_range operation (removes items by value relative rank range, requires MapReturnType).
        #[staticmethod]
        pub fn remove_by_value_relative_rank_range(bin_name: String, value: PythonValue, rank: i64, count: Option<i64>, return_type: MapReturnType) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapRemoveByValueRelativeRankRange(bin_name, value, rank, count, return_type),
            }
        }

        /// Create a Map create operation (creates a map with order).
        #[staticmethod]
        pub fn create(bin_name: String, order: MapOrder) -> Self {
            MapOperation {
                op: OperationType::MapCreate(bin_name, order),
                ctx: None,
            }
        }

        /// Create a Map create_with_index operation (creates map with persisted index).
        #[staticmethod]
        pub fn create_with_index(bin_name: String, order: MapOrder) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapCreateWithIndex(bin_name, order),
            }
        }

        /// Create a Map set_policy operation (sets full map policy including order and persist_index).
        #[staticmethod]
        pub fn set_policy(bin_name: String, policy: MapPolicy) -> Self {
            MapOperation {
                ctx: None,
                op: OperationType::MapSetPolicy(bin_name, policy),
            }
        }

        /// Set the context for this operation. Used for nested CDT operations.
        pub fn set_context(&self, ctx: Vec<CTX>) -> Self {
            MapOperation {
                op: self.op.clone(),
                ctx: Some(ctx),
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  BitOperation
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Bit operations. Create bit operations used by the client's `operate()` method.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct BitOperation {
        pub(crate) op: OperationType,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl BitOperation {
        /// ``True`` when re-sending this operation after an in-doubt attempt
        /// leaves the record as one application would. Reads are idempotent;
        /// every modify operation of this class is not. See
        /// :attr:`Operation.is_idempotent` for the rule.
        #[getter]
        pub fn is_idempotent(&self) -> bool {
            self.op.is_idempotent()
        }

        /// Create a Bit resize operation (resizes byte array, requires BitPolicy).
        #[staticmethod]
        pub fn resize(bin_name: String, byte_size: i64, resize_flags: Option<BitwiseResizeFlags>, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitResize(bin_name, byte_size, resize_flags, policy),
            }
        }
        /// Create a Bit insert operation (inserts bytes, requires BitPolicy).
        #[staticmethod]
        pub fn insert(bin_name: String, byte_offset: i64, value: PythonValue, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitInsert(bin_name, byte_offset, value, policy),
            }
        }

        /// Create a Bit remove operation (removes bytes, requires BitPolicy).
        #[staticmethod]
        pub fn remove(bin_name: String, byte_offset: i64, byte_size: i64, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitRemove(bin_name, byte_offset, byte_size, policy),
            }
        }

        /// Create a Bit set operation (sets bits, requires BitPolicy).
        #[staticmethod]
        pub fn set(bin_name: String, bit_offset: i64, bit_size: i64, value: PythonValue, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitSet(bin_name, bit_offset, bit_size, value, policy),
            }
        }

        /// Create a Bit or operation (performs bitwise OR, requires BitPolicy).
        #[staticmethod]
        #[pyo3(name = "or_")]
        pub fn or(bin_name: String, bit_offset: i64, bit_size: i64, value: PythonValue, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitOr(bin_name, bit_offset, bit_size, value, policy),
            }
        }

        /// Create a Bit xor operation (performs bitwise XOR, requires BitPolicy).
        #[staticmethod]
        pub fn xor(bin_name: String, bit_offset: i64, bit_size: i64, value: PythonValue, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitXor(bin_name, bit_offset, bit_size, value, policy),
            }
        }

        /// Create a Bit and operation (performs bitwise AND, requires BitPolicy).
        #[staticmethod]
        #[pyo3(name = "and_")]
        pub fn and(bin_name: String, bit_offset: i64, bit_size: i64, value: PythonValue, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitAnd(bin_name, bit_offset, bit_size, value, policy),
            }
        }

        /// Create a Bit not operation (performs bitwise NOT, requires BitPolicy).
        #[staticmethod]
        #[pyo3(name = "not_")]
        pub fn not(bin_name: String, bit_offset: i64, bit_size: i64, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitNot(bin_name, bit_offset, bit_size, policy),
            }
        }

        /// Create a Bit lshift operation (performs left shift, requires BitPolicy).
        #[staticmethod]
        pub fn lshift(bin_name: String, bit_offset: i64, bit_size: i64, shift: i64, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitLShift(bin_name, bit_offset, bit_size, shift, policy),
            }
        }

        /// Create a Bit rshift operation (performs right shift, requires BitPolicy).
        #[staticmethod]
        pub fn rshift(bin_name: String, bit_offset: i64, bit_size: i64, shift: i64, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitRShift(bin_name, bit_offset, bit_size, shift, policy),
            }
        }

        /// Create a Bit add operation (adds to integer value, requires BitPolicy).
        #[staticmethod]
        pub fn add(bin_name: String, bit_offset: i64, bit_size: i64, value: i64, signed: bool, action: BitwiseOverflowActions, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitAdd(bin_name, bit_offset, bit_size, value, signed, action, policy),
            }
        }

        /// Create a Bit subtract operation (subtracts from integer value, requires BitPolicy).
        #[staticmethod]
        pub fn subtract(bin_name: String, bit_offset: i64, bit_size: i64, value: i64, signed: bool, action: BitwiseOverflowActions, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitSubtract(bin_name, bit_offset, bit_size, value, signed, action, policy),
            }
        }

        /// Create a Bit set_int operation (sets integer value, requires BitPolicy).
        #[staticmethod]
        pub fn set_int(bin_name: String, bit_offset: i64, bit_size: i64, value: i64, policy: BitPolicy) -> Self {
            BitOperation {
                op: OperationType::BitSetInt(bin_name, bit_offset, bit_size, value, policy),
            }
        }

        /// Create a Bit get operation (gets bits, read-only).
        #[staticmethod]
        pub fn get(bin_name: String, bit_offset: i64, bit_size: i64) -> Self {
            BitOperation {
                op: OperationType::BitGet(bin_name, bit_offset, bit_size),
            }
        }

        /// Create a Bit count operation (counts set bits, read-only).
        #[staticmethod]
        pub fn count(bin_name: String, bit_offset: i64, bit_size: i64) -> Self {
            BitOperation {
                op: OperationType::BitCount(bin_name, bit_offset, bit_size),
            }
        }

        /// Create a Bit lscan operation (scans left for value, read-only).
        #[staticmethod]
        pub fn lscan(bin_name: String, bit_offset: i64, bit_size: i64, value: bool) -> Self {
            BitOperation {
                op: OperationType::BitLScan(bin_name, bit_offset, bit_size, value),
            }
        }

        /// Create a Bit rscan operation (scans right for value, read-only).
        #[staticmethod]
        pub fn rscan(bin_name: String, bit_offset: i64, bit_size: i64, value: bool) -> Self {
            BitOperation {
                op: OperationType::BitRScan(bin_name, bit_offset, bit_size, value),
            }
        }

        /// Create a Bit get_int operation (gets integer value, read-only).
        #[staticmethod]
        pub fn get_int(bin_name: String, bit_offset: i64, bit_size: i64, signed: bool) -> Self {
            BitOperation {
                op: OperationType::BitGetInt(bin_name, bit_offset, bit_size, signed),
            }
        }

        /// Create a Bit b64_encode operation (returns base64 text of the blob, read-only).
        ///
        /// Without a range, encodes the whole blob. With ``byte_offset`` and
        /// ``byte_size`` (required together), encodes that byte range; a negative
        /// ``byte_offset`` counts back from the end of the blob. When
        /// ``invert_size`` is true, ``byte_size`` counts back from the end
        /// instead, so an inverted size of 0 encodes through to the end.
        ///
        /// Requires Aerospike Server version 8.2.0 or later.
        #[staticmethod]
        #[pyo3(signature = (bin_name, byte_offset = None, byte_size = None, invert_size = false))]
        pub fn b64_encode(
            bin_name: String,
            byte_offset: Option<i64>,
            byte_size: Option<i64>,
            invert_size: bool,
        ) -> PyResult<Self> {
            if byte_offset.is_some() != byte_size.is_some() {
                return Err(PyValueError::new_err(
                    "byte_offset and byte_size must be provided together",
                ));
            }
            if invert_size && byte_size.is_none() {
                return Err(PyValueError::new_err(
                    "invert_size requires byte_offset and byte_size",
                ));
            }
            Ok(BitOperation {
                op: OperationType::BitB64Encode(bin_name, byte_offset, byte_size, invert_size),
            })
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  HllOperation
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// HLL (HyperLogLog) operations. Create HLL operations used by the client's `operate()` method.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct HllOperation {
        pub(crate) op: OperationType,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl HllOperation {
        /// ``True`` when re-sending this operation after an in-doubt attempt
        /// leaves the record as one application would. Reads are idempotent;
        /// every modify operation of this class is not. See
        /// :attr:`Operation.is_idempotent` for the rule.
        #[getter]
        pub fn is_idempotent(&self) -> bool {
            self.op.is_idempotent()
        }

        /// Create HLL init operation.
        /// Server creates a new HLL or resets an existing HLL.
        /// Server does not return a value.
        ///
        /// ``flags`` may be an ``HLLWriteFlags`` member, a combined bitmask
        /// (``HLLWriteFlags.X | HLLWriteFlags.Y``), or a raw ``int``.
        #[staticmethod]
        #[pyo3(signature = (bin_name, index_bit_count, min_hash_bit_count = -1, flags = None))]
        pub fn init(
            py: Python<'_>,
            bin_name: String,
            index_bit_count: i64,
            min_hash_bit_count: i64,
            flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let f = match &flags {
                None => 0,
                Some(obj) => crate::cdt::hll_write_flags_from_py(obj.bind(py))?,
            };
            Ok(HllOperation {
                op: OperationType::HllInit(bin_name, index_bit_count, min_hash_bit_count, f),
            })
        }

        /// Create HLL add operation.
        /// Server adds values to HLL set. If HLL bin does not exist and index_bit_count is set,
        /// a new HLL bin will be created.
        /// Server returns number of entries that caused HLL to update a register.
        ///
        /// ``flags`` may be an ``HLLWriteFlags`` member, a combined bitmask
        /// (``HLLWriteFlags.X | HLLWriteFlags.Y``), or a raw ``int``.
        #[staticmethod]
        #[pyo3(signature = (bin_name, values, index_bit_count = -1, min_hash_bit_count = -1, flags = None))]
        pub fn add(
            py: Python<'_>,
            bin_name: String,
            values: Vec<PythonValue>,
            index_bit_count: i64,
            min_hash_bit_count: i64,
            flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let f = match &flags {
                None => 0,
                Some(obj) => crate::cdt::hll_write_flags_from_py(obj.bind(py))?,
            };
            Ok(HllOperation {
                op: OperationType::HllAdd(bin_name, values, index_bit_count, min_hash_bit_count, f),
            })
        }

        /// Create HLL get_count operation.
        /// Server returns estimated number of elements in the HLL bin.
        #[staticmethod]
        pub fn get_count(bin_name: String) -> Self {
            HllOperation {
                op: OperationType::HllGetCount(bin_name),
            }
        }

        /// Create HLL describe operation.
        /// Server returns index_bit_count and min_hash_bit_count used to create HLL bin
        /// in a list of longs. The list size is 2.
        #[staticmethod]
        pub fn describe(bin_name: String) -> Self {
            HllOperation {
                op: OperationType::HllDescribe(bin_name),
            }
        }

        /// Create HLL refresh_count operation.
        /// Server updates the cached count (if stale) and returns the count.
        #[staticmethod]
        pub fn refresh_count(bin_name: String) -> Self {
            HllOperation {
                op: OperationType::HllRefreshCount(bin_name),
            }
        }

        /// Create HLL fold operation.
        /// Server folds index_bit_count to the specified value.
        /// This can only be applied when min_hash_bit_count on the HLL bin is 0.
        /// Server does not return a value.
        #[staticmethod]
        pub fn fold(bin_name: String, index_bit_count: i64) -> Self {
            HllOperation {
                op: OperationType::HllFold(bin_name, index_bit_count),
            }
        }

        /// Create HLL get_union operation.
        /// Server returns an HLL object that is the union of all specified HLL objects
        /// in the list with the HLL bin.
        #[staticmethod]
        pub fn get_union(bin_name: String, hll_list: Vec<PythonValue>) -> Self {
            HllOperation {
                op: OperationType::HllGetUnion(bin_name, hll_list),
            }
        }

        /// Create HLL get_union_count operation.
        /// Server returns estimated number of elements that would be contained
        /// by the union of these HLL objects.
        #[staticmethod]
        pub fn get_union_count(bin_name: String, hll_list: Vec<PythonValue>) -> Self {
            HllOperation {
                op: OperationType::HllGetUnionCount(bin_name, hll_list),
            }
        }

        /// Create HLL get_intersect_count operation.
        /// Server returns estimated number of elements that would be contained
        /// by the intersection of these HLL objects.
        #[staticmethod]
        pub fn get_intersect_count(bin_name: String, hll_list: Vec<PythonValue>) -> Self {
            HllOperation {
                op: OperationType::HllGetIntersectCount(bin_name, hll_list),
            }
        }

        /// Create HLL get_similarity operation.
        /// Server returns estimated similarity of these HLL objects. Return type is a double.
        #[staticmethod]
        pub fn get_similarity(bin_name: String, hll_list: Vec<PythonValue>) -> Self {
            HllOperation {
                op: OperationType::HllGetSimilarity(bin_name, hll_list),
            }
        }

        /// Create HLL set_union operation.
        /// Server sets union of specified HLL objects with HLL bin.
        /// Server does not return a value.
        ///
        /// ``flags`` may be an ``HLLWriteFlags`` member, a combined bitmask
        /// (``HLLWriteFlags.X | HLLWriteFlags.Y``), or a raw ``int``.
        #[staticmethod]
        #[pyo3(signature = (bin_name, hll_list, flags = None))]
        pub fn set_union(
            py: Python<'_>,
            bin_name: String,
            hll_list: Vec<PythonValue>,
            flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let f = match &flags {
                None => 0,
                Some(obj) => crate::cdt::hll_write_flags_from_py(obj.bind(py))?,
            };
            Ok(HllOperation {
                op: OperationType::HllSetUnion(bin_name, hll_list, f),
            })
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ExpOperation
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Expression operations. Create expression operations used by the client's `operate()` method.
    /// Expression operations allow evaluating expressions on the server and optionally storing
    /// the result in a bin.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct ExpOperation {
        pub(crate) op: OperationType,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl ExpOperation {
        /// ``True`` when re-sending this operation after an in-doubt attempt
        /// leaves the record as one application would. Reads are idempotent;
        /// every modify operation of this class is not. See
        /// :attr:`Operation.is_idempotent` for the rule.
        #[getter]
        pub fn is_idempotent(&self) -> bool {
            self.op.is_idempotent()
        }

        /// Create expression read operation.
        ///
        /// Evaluates the expression and returns the result. The result is returned
        /// in the record bins with the specified name.
        ///
        /// Args:
        ///     name: Name to assign to the expression result in the returned record.
        ///     exp: Expression to evaluate.
        ///     flags: Expression read flags (default: ExpReadFlags.DEFAULT). May
        ///         be an ``ExpReadFlags`` member, a combined bitmask
        ///         (``ExpReadFlags.X | ExpReadFlags.Y``), or a raw ``int``.
        ///
        /// Returns:
        ///     An ExpOperation to use with client.operate().
        #[staticmethod]
        #[pyo3(signature = (name, exp, flags = None))]
        pub fn read(
            py: Python<'_>,
            name: String,
            exp: FilterExpression,
            flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let f = match &flags {
                None => 0i64,
                Some(obj) => crate::expressions::exp_read_flags_from_py(obj.bind(py))?,
            };
            Ok(ExpOperation {
                op: OperationType::ExpRead(name, exp, f),
            })
        }

        /// Create expression write operation.
        ///
        /// Evaluates the expression and writes the result to the specified bin.
        ///
        /// Args:
        ///     bin_name: Name of bin to store expression result.
        ///     exp: Expression to evaluate.
        ///     flags: Expression write flags (default: ExpWriteFlags.DEFAULT). May
        ///         be an ``ExpWriteFlags`` member, a combined bitmask
        ///         (``ExpWriteFlags.X | ExpWriteFlags.Y``), or a raw ``int``.
        ///
        /// Returns:
        ///     An ExpOperation to use with client.operate().
        #[staticmethod]
        #[pyo3(signature = (bin_name, exp, flags = None))]
        pub fn write(
            py: Python<'_>,
            bin_name: String,
            exp: FilterExpression,
            flags: Option<Py<PyAny>>,
        ) -> PyResult<Self> {
            let f = match &flags {
                None => 0i64,
                Some(obj) => crate::expressions::exp_write_flags_from_py(obj.bind(py))?,
            };
            Ok(ExpOperation {
                op: OperationType::ExpWrite(bin_name, exp, f),
            })
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  CdtOperation
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// CDT path expression operations for reading and writing nested CDT data.
    ///
    /// Requires Aerospike Server version >= 8.1.1.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct CdtOperation {
        pub(crate) op: OperationType,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl CdtOperation {
        /// ``True`` when re-sending this operation after an in-doubt attempt
        /// leaves the record as one application would. Reads are idempotent;
        /// every modify operation of this class is not. See
        /// :attr:`Operation.is_idempotent` for the rule.
        #[getter]
        pub fn is_idempotent(&self) -> bool {
            self.op.is_idempotent()
        }

        /// Create a CDT path select operation.
        ///
        /// Reads nested CDT data identified by the given path context, returning
        /// result elements controlled by ``flag``.
        ///
        /// Args:
        ///     bin_name: Name of the bin containing the top-level CDT.
        ///     flag: Controls what is returned. Pass any combination of
        ///         ``SelectFlags`` constants (e.g. ``SelectFlags.VALUE |
        ///         SelectFlags.NO_FAIL``) — the resulting value is a plain
        ///         ``int`` bitmask.
        ///     ctx: Path into the CDT — one ``CTX`` per nesting level.
        ///
        /// Returns:
        ///     A ``CdtOperation`` to pass to ``client.operate()``.
        ///
        /// Example::
        ///
        ///     from aerospike_native import CdtOperation, CTX, SelectFlags
        ///     op = CdtOperation.select_by_path(
        ///         "inventory",
        ///         SelectFlags.VALUE,
        ///         [CTX.map_key("books"), CTX.list_index(0)],
        ///     )
        #[staticmethod]
        pub fn select_by_path(
            bin_name: String,
            flag: i64,
            ctx: Vec<CTX>,
        ) -> Self {
            CdtOperation {
                op: OperationType::CdtSelectByPath(bin_name, flag, ctx),
            }
        }

        /// Create a CDT path modify operation.
        ///
        /// Writes to nested CDT data identified by the given path context, using
        /// ``exp`` to compute the new value.
        ///
        /// Args:
        ///     bin_name: Name of the bin containing the top-level CDT.
        ///     flag: Controls error-handling behavior. Pass any combination of
        ///         ``ModifyFlags`` constants — the resulting value is a plain
        ///         ``int`` bitmask.
        ///     exp: Expression that produces the value to write.
        ///     ctx: Path into the CDT — one ``CTX`` per nesting level.
        ///
        /// Returns:
        ///     A ``CdtOperation`` to pass to ``client.operate()``.
        ///
        /// Example::
        ///
        ///     from aerospike_native import CdtOperation, CTX, ModifyFlags, FilterExpression
        ///     price_path = [CTX.map_key("books"), CTX.list_rank(0), CTX.map_key("price")]
        ///     new_price = FilterExpression.float_val(9.99)
        ///     op = CdtOperation.modify_by_path(
        ///         "inventory",
        ///         ModifyFlags.DEFAULT,
        ///         new_price,
        ///         price_path,
        ///     )
        #[staticmethod]
        pub fn modify_by_path(
            bin_name: String,
            flag: i64,
            exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            CdtOperation {
                op: OperationType::CdtModifyByPath(bin_name, flag, exp, ctx),
            }
        }

        // ===== Convenience builders on top of select_by_path / modify_by_path =====
        //
        // Each preset corresponds to a common (flag, op-kind) pairing so callers
        // don't have to remember the SelectFlags / ModifyFlags constants for the
        // most frequent shapes of path query.

        /// Select the *values* at every path-resolved location
        /// (``SelectFlags.VALUE``). Equivalent to
        /// ``CdtOperation.select_by_path(bin, SelectFlags.VALUE, ctx)``.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn select_values(bin_name: String, ctx: Vec<CTX>) -> Self {
            CdtOperation {
                op: OperationType::CdtSelectByPath(
                    bin_name,
                    aerospike_core::operations::path::SelectFlag::VALUE.bits(),
                    ctx,
                ),
            }
        }

        /// Select the matching *map keys* (``SelectFlags.MAP_KEY``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn select_map_keys(bin_name: String, ctx: Vec<CTX>) -> Self {
            CdtOperation {
                op: OperationType::CdtSelectByPath(
                    bin_name,
                    aerospike_core::operations::path::SelectFlag::MAP_KEY.bits(),
                    ctx,
                ),
            }
        }

        /// Select map *key/value pairs* (``SelectFlags.MAP_KEY_VALUE``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn select_map_entries(bin_name: String, ctx: Vec<CTX>) -> Self {
            CdtOperation {
                op: OperationType::CdtSelectByPath(
                    bin_name,
                    aerospike_core::operations::path::SelectFlag::MAP_KEY_VALUE.bits(),
                    ctx,
                ),
            }
        }

        /// Select the *original tree shape* preserving only matching nodes
        /// (``SelectFlags.MATCHING_TREE``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn select_matching_tree(bin_name: String, ctx: Vec<CTX>) -> Self {
            CdtOperation {
                op: OperationType::CdtSelectByPath(
                    bin_name,
                    aerospike_core::operations::path::SelectFlag::MATCHING_TREE.bits(),
                    ctx,
                ),
            }
        }

        /// Modify with default flags, failing on type mismatches
        /// (``ModifyFlags.DEFAULT``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn modify(bin_name: String, exp: FilterExpression, ctx: Vec<CTX>) -> Self {
            CdtOperation {
                op: OperationType::CdtModifyByPath(
                    bin_name,
                    aerospike_core::operations::path::ModifyFlag::DEFAULT.bits(),
                    exp,
                    ctx,
                ),
            }
        }

        /// Modify with ``ModifyFlags.NO_FAIL`` so type-mismatched leaves are
        /// silently skipped instead of aborting the whole operation.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn modify_no_fail(bin_name: String, exp: FilterExpression, ctx: Vec<CTX>) -> Self {
            CdtOperation {
                op: OperationType::CdtModifyByPath(
                    bin_name,
                    aerospike_core::operations::path::ModifyFlag::NO_FAIL.bits(),
                    exp,
                    ctx,
                ),
            }
        }

        /// Remove the leaves resolved by a path. Equivalent to
        /// ``CdtOperation.modify_by_path(bin, ModifyFlags.DEFAULT, FilterExpression.remove_result(), ctx)``.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn remove(bin_name: String, ctx: Vec<CTX>) -> Self {
            CdtOperation {
                op: OperationType::CdtModifyByPath(
                    bin_name,
                    aerospike_core::operations::path::ModifyFlag::DEFAULT.bits(),
                    FilterExpression {
                        _as: aerospike_core::expressions::exp_remove_result(),
                    },
                    ctx,
                ),
            }
        }
    }

pub(crate) fn bins_flag(bins: Option<Vec<String>>) -> aerospike_core::Bins {
    match bins {
        None => aerospike_core::Bins::All,
        Some(bins) => {
            if !bins.is_empty() {
                aerospike_core::Bins::Some(bins)
            } else {
                aerospike_core::Bins::None
            }
        }
    }
}

/// Convert a Python ``{bin_name: value}`` mapping into core bins, rejecting
/// non-string bin names.
///
/// Every write entry point converts its payload up front like this because the
/// conversion needs the GIL, while the op it feeds runs on a Tokio worker.
pub(crate) fn bins_from_dict(
    bins: &Bound<'_, pyo3::types::PyDict>,
) -> PyResult<Vec<aerospike_core::Bin>> {
    let mut bin_vec = Vec::with_capacity(bins.len());
    for (py_key, py_val) in bins.iter() {
        let name = py_key.extract::<String>().map_err(|_| {
            PyTypeError::new_err("A bin name must be a string or unicode string")
        })?;
        let val: PythonValue = py_val.extract()?;
        bin_vec.push(aerospike_core::Bin::new(name, val.into()));
    }
    Ok(bin_vec)
}

/// Positional [`bins_from_dict`] for the per-key-payload entry points: each
/// object must be a mapping, and slot *i* becomes the payload for key *i*.
pub(crate) fn bins_from_dict_list(
    py: Python<'_>,
    bins_list: &[Py<PyAny>],
) -> PyResult<Vec<Vec<aerospike_core::Bin>>> {
    bins_list
        .iter()
        .map(|obj| bins_from_dict(obj.bind(py).cast::<pyo3::types::PyDict>()?))
        .collect()
}

/// Operation plus optional nested CDT context from the Python operation wrappers.
#[derive(Clone, Debug)]
pub(crate) struct OpWithCtx {
    pub op: OperationType,
    pub ctx: Option<Vec<aerospike_core::operations::cdt_context::CdtContext>>,
}

/// Extract Python operation objects into `OpWithCtx` (preserves list/map CDT context).
pub(crate) fn extract_py_ops_with_ctx(py: Python<'_>, py_ops: &[Py<PyAny>]) -> PyResult<Vec<OpWithCtx>> {
    let mut rust_ops = Vec::with_capacity(py_ops.len());
    for op_obj in py_ops {
        if let Ok(py_op) = op_obj.extract::<PyRef<Operation>>(py) {
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx: None,
            });
        } else if let Ok(py_op) = op_obj.extract::<PyRef<ListOperation>>(py) {
            let ctx = py_op.ctx.as_ref().map(|ctx_vec| {
                ctx_vec.iter().map(|c| c.ctx.clone()).collect()
            });
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx,
            });
        } else if let Ok(py_op) = op_obj.extract::<PyRef<MapOperation>>(py) {
            let ctx = py_op.ctx.as_ref().map(|ctx_vec| {
                ctx_vec.iter().map(|c| c.ctx.clone()).collect()
            });
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx,
            });
        } else if let Ok(py_op) = op_obj.extract::<PyRef<BitOperation>>(py) {
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx: None,
            });
        } else if let Ok(py_op) = op_obj.extract::<PyRef<StringOperation>>(py) {
            let ctx = py_op.ctx.as_ref().map(|ctx_vec| {
                ctx_vec.iter().map(|c| c.ctx.clone()).collect()
            });
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx,
            });
        } else if let Ok(py_op) = op_obj.extract::<PyRef<HllOperation>>(py) {
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx: None,
            });
        } else if let Ok(py_op) = op_obj.extract::<PyRef<ExpOperation>>(py) {
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx: None,
            });
        } else if let Ok(py_op) = op_obj.extract::<PyRef<CdtOperation>>(py) {
            rust_ops.push(OpWithCtx {
                op: py_op.op.clone(),
                ctx: None,
            });
        } else {
            return Err(PyTypeError::new_err(
                "Operation must be Operation, ListOperation, MapOperation, BitOperation, StringOperation, HllOperation, ExpOperation, or CdtOperation"
            ));
        }
    }
    Ok(rust_ops)
}

pub(crate) fn record_batch_ops_have_write(rust_ops: &[OpWithCtx]) -> bool {
    for owc in rust_ops {
        match &owc.op {
            OperationType::Put(_, _) | OperationType::Add(_, _) | OperationType::Append(_, _) |
            OperationType::Prepend(_, _) | OperationType::Delete() | OperationType::Touch() |
            OperationType::ListSet(_, _, _) | OperationType::ListSetWithPolicy(_, _, _, _) |
            OperationType::ListAppend(_, _, _) | OperationType::ListAppendItems(_, _, _) |
            OperationType::ListInsert(_, _, _, _) | OperationType::ListInsertItems(_, _, _, _) |
            OperationType::ListIncrement(_, _, _, _) | OperationType::ListIncrementByOne(_, _) |
            OperationType::ListIncrementByOneWithPolicy(_, _, _) |
            OperationType::ListSort(_, _) | OperationType::ListSetOrder(_, _) |
            OperationType::ListCreateWithIndex(_, _) | OperationType::ListSetOrderWithIndex(_, _) |
            OperationType::ListRemove(_, _) | OperationType::ListRemoveRange(_, _, _) |
            OperationType::ListRemoveRangeFrom(_, _) | OperationType::ListPop(_, _) |
            OperationType::ListPopRange(_, _, _) | OperationType::ListPopRangeFrom(_, _) |
            OperationType::ListTrim(_, _, _) | OperationType::ListClear(_) |
            OperationType::ListCreate(_, _, _, _) |
            OperationType::ListRemoveByIndex(_, _, _) | OperationType::ListRemoveByIndexRange(_, _, _, _) |
            OperationType::ListRemoveByRank(_, _, _) | OperationType::ListRemoveByRankRange(_, _, _, _) |
            OperationType::ListRemoveByValue(_, _, _) | OperationType::ListRemoveByValueList(_, _, _) |
            OperationType::ListRemoveByValueRange(_, _, _, _) |
            OperationType::ListRemoveByValueRelativeRankRange(_, _, _, _, _) |
            OperationType::MapPut(_, _, _, _) | OperationType::MapPutItems(_, _, _) |
            OperationType::MapIncrementValue(_, _, _, _) | OperationType::MapDecrementValue(_, _, _, _) |
            OperationType::MapClear(_) | OperationType::MapSetMapPolicy(_, _) |
            OperationType::MapSetPolicy(_, _) | OperationType::MapCreate(_, _) |
            OperationType::MapCreateWithIndex(_, _) | OperationType::MapRemoveByKey(_, _, _) |
            OperationType::MapRemoveByKeyRange(_, _, _, _) | OperationType::MapRemoveByIndex(_, _, _) |
            OperationType::MapRemoveByIndexRange(_, _, _, _) |
            OperationType::MapRemoveByIndexRangeFrom(_, _, _) | OperationType::MapRemoveByRank(_, _, _) |
            OperationType::MapRemoveByRankRange(_, _, _, _) |
            OperationType::MapRemoveByRankRangeFrom(_, _, _) | OperationType::MapRemoveByValue(_, _, _) |
            OperationType::MapRemoveByValueRange(_, _, _, _) | OperationType::MapRemoveByKeyList(_, _, _) |
            OperationType::MapRemoveByValueList(_, _, _) |
            OperationType::MapRemoveByKeyRelativeIndexRange(_, _, _, _, _) |
            OperationType::MapRemoveByValueRelativeRankRange(_, _, _, _, _) |
            OperationType::BitResize(_, _, _, _) | OperationType::BitInsert(_, _, _, _) |
            OperationType::BitRemove(_, _, _, _) | OperationType::BitSet(_, _, _, _, _) |
            OperationType::BitOr(_, _, _, _, _) | OperationType::BitXor(_, _, _, _, _) |
            OperationType::BitAnd(_, _, _, _, _) | OperationType::BitNot(_, _, _, _) |
            OperationType::BitLShift(_, _, _, _, _) | OperationType::BitRShift(_, _, _, _, _) |
            OperationType::BitAdd(_, _, _, _, _, _, _) | OperationType::BitSubtract(_, _, _, _, _, _, _) |
            OperationType::BitSetInt(_, _, _, _, _) |
            OperationType::ExpWrite(_, _, _) |
            OperationType::HllInit(_, _, _, _) | OperationType::HllAdd(_, _, _, _, _) |
            OperationType::HllFold(_, _) | OperationType::HllSetUnion(_, _, _) |
            OperationType::CdtModifyByPath(_, _, _, _) |
            // STRING_MODIFY ops (sub-ops 50..66) — every string-mutating op flips this
            // to `true` so batch routing picks the write path. Note: `StringToString`
            // is a READ op (TO_STRING op-type 19) and is intentionally excluded.
            OperationType::StringInsert(_, _, _, _) | OperationType::StringOverwrite(_, _, _, _) |
            OperationType::StringConcat(_, _, _) | OperationType::StringAppend(_, _, _) |
            OperationType::StringPrepend(_, _, _) | OperationType::StringSnip(_, _, _, _) |
            OperationType::StringReplace(_, _, _, _) | OperationType::StringReplaceAll(_, _, _, _) |
            OperationType::StringUpper(_, _) | OperationType::StringLower(_, _) |
            OperationType::StringCaseFold(_, _) | OperationType::StringNormalizeNfc(_, _) |
            OperationType::StringTrimStart(_, _) | OperationType::StringTrimEnd(_, _) |
            OperationType::StringTrim(_, _) |
            OperationType::StringPadStart(_, _, _, _) | OperationType::StringPadEnd(_, _, _, _) |
            OperationType::StringRepeat(_, _, _) | OperationType::StringRegexReplace(_, _, _, _, _) => {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Full operation conversion (scalar, CDT, expressions, HLL) with optional CDT context.
/// When `disallow_hll` is true, returns `NotImplementedError` if any HLL operation is present
/// (`batch_operate` policy).
pub(crate) fn convert_ops_with_ctx_to_core(
    rust_ops: &[OpWithCtx],
    disallow_hll: bool,
) -> PyResult<(Vec<aerospike_core::operations::Operation>, bool)> {

    if disallow_hll {
        for owc in rust_ops {
            match &owc.op {
                OperationType::HllInit(_, _, _, _)
                | OperationType::HllAdd(_, _, _, _, _)
                | OperationType::HllGetCount(_)
                | OperationType::HllDescribe(_)
                | OperationType::HllRefreshCount(_)
                | OperationType::HllFold(_, _)
                | OperationType::HllGetUnion(_, _)
                | OperationType::HllGetUnionCount(_, _)
                | OperationType::HllGetIntersectCount(_, _)
                | OperationType::HllGetSimilarity(_, _)
                | OperationType::HllSetUnion(_, _, _) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
                        "HLL operations are not supported in batch_operate. Use operate() instead.",
                    ));
                }
                _ => {}
            }
        }
    }

    use aerospike_core::operations;

    // First pass: collect all bins/values that need to live as long as the operations
    let mut bin_storage: Vec<aerospike_core::Bin> = Vec::new();
    let mut value_storage: Vec<aerospike_core::Value> = Vec::new();
    let mut map_storage: Vec<HashMap<aerospike_core::Value, aerospike_core::Value>> = Vec::new();
    let mut list_storage: Vec<Vec<aerospike_core::Value>> = Vec::new();
    let mut hll_value_storage: Vec<Vec<aerospike_core::Value>> = Vec::new();
    for op_with_ctx in rust_ops {
        match &op_with_ctx.op {
            OperationType::Put(bin_name, value) |
            OperationType::Add(bin_name, value) |
            OperationType::Append(bin_name, value) |
            OperationType::Prepend(bin_name, value) => {
                let bin = aerospike_core::Bin::new(bin_name.clone(), value.clone().into());
                bin_storage.push(bin);
            }
            OperationType::ListSet(_, _, value) => {
                // Store the value for list_set operation
                value_storage.push(value.clone().into());
            }
            OperationType::ListSetWithPolicy(_, _, value, _) => {
                // Store the value for list set_with_policy operation
                value_storage.push(value.clone().into());
            }
            OperationType::ListAppend(_, value, _) => {
                // Store the value for list_append operation
                value_storage.push(value.clone().into());
            }
            OperationType::ListAppendItems(_, values, _) => {
                // Store all values for list_append_items operation
                for value in values {
                    value_storage.push(value.clone().into());
                }
            }
            OperationType::ListInsert(_, _, value, _) => {
                // Store the value for list_insert operation
                value_storage.push(value.clone().into());
            }
            OperationType::ListInsertItems(_, _, values, _) => {
                // Store all values for list_insert_items operation
                for value in values {
                    value_storage.push(value.clone().into());
                }
            }
            OperationType::ListGetByValue(_, value, _) => {
                // Store value for list get_by_value operation
                value_storage.push(value.clone().into());
            }
            OperationType::ListGetByValueRange(_, begin, end, _) => {
                // Store begin and end values for list get_by_value_range operation
                value_storage.push(begin.clone().into());
                value_storage.push(end.clone().into());
            }
            OperationType::ListGetByValueList(_, values, _) => {
                // Store list of values for list get_by_value_list operation
                let mut value_list = Vec::new();
                for value in values {
                    value_list.push(value.clone().into());
                }
                list_storage.push(value_list);
            }
            OperationType::ListGetByValueRelativeRankRange(_, value, _, _, _) => {
                // Store value for list get_by_value_relative_rank_range operation
                value_storage.push(value.clone().into());
            }
            OperationType::ListRemoveByValue(_, value, _) => {
                // Store value for list remove_by_value operation
                value_storage.push(value.clone().into());
            }
            OperationType::ListRemoveByValueList(_, values, _) => {
                // Store list of values for list remove_by_value_list operation
                let mut value_list = Vec::new();
                for value in values {
                    value_list.push(value.clone().into());
                }
                list_storage.push(value_list);
            }
            OperationType::ListRemoveByValueRange(_, begin, end, _) => {
                // Store begin and end values for list remove_by_value_range operation
                value_storage.push(begin.clone().into());
                value_storage.push(end.clone().into());
            }
            OperationType::ListRemoveByValueRelativeRankRange(_, value, _, _, _) => {
                // Store value for list remove_by_value_relative_rank_range operation
                value_storage.push(value.clone().into());
            }
            OperationType::MapPut(_, key, value, _) => {
                // Store key and value for map_put operation
                value_storage.push(key.clone().into());
                value_storage.push(value.clone().into());
            }
            OperationType::MapPutItems(_, items, _) => {
                // Store all keys and values for map_put_items operation
                use std::collections::HashMap;
                let mut map = HashMap::new();
                for (key, value) in items {
                    map.insert(key.clone().into(), value.clone().into());
                }
                map_storage.push(map);
            }
            OperationType::MapIncrementValue(_, key, value, _) | OperationType::MapDecrementValue(_, key, value, _) => {
                // Store key and increment/decrement value for map increment/decrement operations
                value_storage.push(key.clone().into());
                value_storage.push(aerospike_core::Value::Int(*value));
            }
            OperationType::MapGetByKey(_, key, _) | OperationType::MapRemoveByKey(_, key, _) => {
                // Store key for map get_by_key and remove_by_key operations
                value_storage.push(key.clone().into());
            }
            OperationType::MapGetByKeyRange(_, begin, end, _) | OperationType::MapRemoveByKeyRange(_, begin, end, _) => {
                // Store begin and end keys for map get_by_key_range and remove_by_key_range operations
                value_storage.push(begin.clone().into());
                value_storage.push(end.clone().into());
            }
            OperationType::MapGetByValue(_, value, _) | OperationType::MapRemoveByValue(_, value, _) => {
                // Store value for map get_by_value and remove_by_value operations
                value_storage.push(value.clone().into());
            }
            OperationType::MapGetByValueRange(_, begin, end, _) | OperationType::MapRemoveByValueRange(_, begin, end, _) => {
                // Store begin and end values for map get_by_value_range and remove_by_value_range operations
                value_storage.push(begin.clone().into());
                value_storage.push(end.clone().into());
            }
            OperationType::MapGetByKeyList(_, keys, _) | OperationType::MapRemoveByKeyList(_, keys, _) => {
                // Store list of keys for map get_by_key_list and remove_by_key_list operations
                let mut key_list = Vec::new();
                for key in keys {
                    key_list.push(key.clone().into());
                }
                list_storage.push(key_list);
            }
            OperationType::MapGetByValueList(_, values, _) | OperationType::MapRemoveByValueList(_, values, _) => {
                // Store list of values for map get_by_value_list and remove_by_value_list operations
                let mut value_list = Vec::new();
                for value in values {
                    value_list.push(value.clone().into());
                }
                list_storage.push(value_list);
            }
            OperationType::MapGetByKeyRelativeIndexRange(_, key, _, _, _) | OperationType::MapRemoveByKeyRelativeIndexRange(_, key, _, _, _) => {
                // Store key for map get_by_key_relative_index_range and remove_by_key_relative_index_range operations
                value_storage.push(key.clone().into());
            }
            OperationType::MapGetByValueRelativeRankRange(_, value, _, _, _) | OperationType::MapRemoveByValueRelativeRankRange(_, value, _, _, _) => {
                // Store value for map get_by_value_relative_rank_range and remove_by_value_relative_rank_range operations
                value_storage.push(value.clone().into());
            }
            OperationType::BitInsert(_, _, value, _) | OperationType::BitSet(_, _, _, value, _) |
            OperationType::BitOr(_, _, _, value, _) | OperationType::BitXor(_, _, _, value, _) |
            OperationType::BitAnd(_, _, _, value, _) => {
                // Store value for bit operations that require a value
                value_storage.push(value.clone().into());
            }
            // Operations that don't require storage in first pass
            OperationType::Get() | OperationType::GetBin(_) | OperationType::GetHeader() |
            OperationType::Delete() | OperationType::Touch() |
            OperationType::ListGet(_, _) | OperationType::ListSize(_) | OperationType::ListJoin(_, _) |
            OperationType::ListPop(_, _) |
            OperationType::ListClear(_) | OperationType::ListGetRange(_, _, _) |
            OperationType::ListRemove(_, _) | OperationType::ListRemoveRange(_, _, _) |
            OperationType::ListGetRangeFrom(_, _) | OperationType::ListPopRange(_, _, _) |
            OperationType::ListPopRangeFrom(_, _) | OperationType::ListRemoveRangeFrom(_, _) |
            OperationType::ListTrim(_, _, _) | OperationType::ListIncrement(_, _, _, _) |
            OperationType::ListIncrementByOne(_, _) | OperationType::ListIncrementByOneWithPolicy(_, _, _) |
            OperationType::ListSort(_, _) | OperationType::ListSetOrder(_, _) |
            OperationType::ListCreateWithIndex(_, _) | OperationType::ListSetOrderWithIndex(_, _) |
            OperationType::ListGetByIndex(_, _, _) | OperationType::ListGetByIndexRange(_, _, _, _) |
            OperationType::ListGetByRank(_, _, _) | OperationType::ListGetByRankRange(_, _, _, _) |
            OperationType::ListRemoveByIndex(_, _, _) | OperationType::ListRemoveByIndexRange(_, _, _, _) |
            OperationType::ListRemoveByRank(_, _, _) | OperationType::ListRemoveByRankRange(_, _, _, _) |
            OperationType::ListCreate(_, _, _, _) |
            OperationType::MapSize(_) | OperationType::MapClear(_) |
            OperationType::MapCreateWithIndex(_, _) | OperationType::MapSetPolicy(_, _) |
            OperationType::MapGetByIndex(_, _, _) | OperationType::MapRemoveByIndex(_, _, _) |
            OperationType::MapGetByIndexRange(_, _, _, _) | OperationType::MapRemoveByIndexRange(_, _, _, _) |
            OperationType::MapGetByIndexRangeFrom(_, _, _) | OperationType::MapRemoveByIndexRangeFrom(_, _, _) |
            OperationType::MapGetByRank(_, _, _) | OperationType::MapRemoveByRank(_, _, _) |
            OperationType::MapGetByRankRange(_, _, _, _) | OperationType::MapRemoveByRankRange(_, _, _, _) |
            OperationType::MapGetByRankRangeFrom(_, _, _) | OperationType::MapRemoveByRankRangeFrom(_, _, _) |
            OperationType::MapSetMapPolicy(_, _) | OperationType::MapCreate(_, _) |
            OperationType::BitResize(_, _, _, _) | OperationType::BitRemove(_, _, _, _) |
            OperationType::BitNot(_, _, _, _) | OperationType::BitLShift(_, _, _, _, _) |
            OperationType::BitRShift(_, _, _, _, _) | OperationType::BitAdd(_, _, _, _, _, _, _) |
            OperationType::BitSubtract(_, _, _, _, _, _, _) | OperationType::BitSetInt(_, _, _, _, _) |
            OperationType::BitGet(_, _, _) | OperationType::BitCount(_, _, _) |
            OperationType::BitLScan(_, _, _, _) | OperationType::BitRScan(_, _, _, _) |
            OperationType::BitGetInt(_, _, _, _) |
            OperationType::BitB64Encode(_, _, _, _) |
            OperationType::HllInit(_, _, _, _) |
            OperationType::HllGetCount(_) | OperationType::HllDescribe(_) |
            OperationType::HllRefreshCount(_) | OperationType::HllFold(_, _) => {
            }
            OperationType::HllAdd(_, values, _, _, _) => {
                let core_values: Vec<aerospike_core::Value> = values.iter().map(|v| v.clone().into()).collect();
                hll_value_storage.push(core_values);
            }
            OperationType::HllGetUnion(_, hll_list) |
            OperationType::HllGetUnionCount(_, hll_list) |
            OperationType::HllGetIntersectCount(_, hll_list) |
            OperationType::HllGetSimilarity(_, hll_list) => {
                let core_values: Vec<aerospike_core::Value> = hll_list.iter().map(|v| v.clone().into()).collect();
                hll_value_storage.push(core_values);
            }
            OperationType::HllSetUnion(_, hll_list, _) => {
                let core_values: Vec<aerospike_core::Value> = hll_list.iter().map(|v| v.clone().into()).collect();
                hll_value_storage.push(core_values);
            }
            // Expression and CDT path operations clone directly - no pre-storage needed
            OperationType::ExpRead(_, _, _) | OperationType::ExpWrite(_, _, _)
            | OperationType::CdtSelectByPath(_, _, _)
            | OperationType::CdtModifyByPath(_, _, _, _) => {}

            // String ops (server 8.2.0+): args are owned in the variant; rust-core's
            // builders take &str and copy/own internally. No pre-storage needed.
            OperationType::StringStrlen(_) | OperationType::StringSubstr(_, _, _)
            | OperationType::StringCharAt(_, _) | OperationType::StringFind(_, _, _)
            | OperationType::StringContains(_, _) | OperationType::StringStartsWith(_, _)
            | OperationType::StringEndsWith(_, _) | OperationType::StringToInteger(_)
            | OperationType::StringToDouble(_) | OperationType::StringByteLength(_)
            | OperationType::StringIsNumeric(_, _) | OperationType::StringIsUpper(_)
            | OperationType::StringIsLower(_) | OperationType::StringToBlob(_)
            | OperationType::StringSplit(_, _) | OperationType::StringB64Decode(_)
            | OperationType::StringRegexCompare(_, _, _)
            | OperationType::StringInsert(_, _, _, _) | OperationType::StringOverwrite(_, _, _, _)
            | OperationType::StringConcat(_, _, _) | OperationType::StringAppend(_, _, _)
            | OperationType::StringPrepend(_, _, _) | OperationType::StringSnip(_, _, _, _)
            | OperationType::StringReplace(_, _, _, _) | OperationType::StringReplaceAll(_, _, _, _)
            | OperationType::StringUpper(_, _) | OperationType::StringLower(_, _)
            | OperationType::StringCaseFold(_, _) | OperationType::StringNormalizeNfc(_, _)
            | OperationType::StringTrimStart(_, _) | OperationType::StringTrimEnd(_, _)
            | OperationType::StringTrim(_, _)
            | OperationType::StringPadStart(_, _, _, _) | OperationType::StringPadEnd(_, _, _, _)
            | OperationType::StringRepeat(_, _, _) | OperationType::StringRegexReplace(_, _, _, _, _)
            | OperationType::StringToString(_) => {}
        }
    }

    // Second pass: convert operations, using references to stored bins/values
    let mut bin_idx = 0;
    let mut value_idx = 0;
    let mut map_idx = 0;
    let mut list_idx = 0;
    let mut hll_idx = 0;
    let mut core_ops: Vec<operations::Operation> = Vec::new();

    for op_with_ctx in rust_ops {
        let core_op = match &op_with_ctx.op {
            OperationType::Get() => {
                // Use the operations module's get() function to create a Get operation
                operations::get()
            }
            OperationType::GetBin(bin_name) => {
                // Use the operations module's get_bin() function to get a specific bin
                operations::get_bin(bin_name)
            }
            OperationType::GetHeader() => {
                // Use the operations module's get_header() function
                operations::get_header()
            }
            OperationType::Put(_, _) => {
                // Use a reference to the stored bin
                let op = operations::put(&bin_storage[bin_idx]);
                bin_idx += 1;
                op
            }
            OperationType::Add(_, _) => {
                // Use a reference to the stored bin
                let op = operations::add(&bin_storage[bin_idx]);
                bin_idx += 1;
                op
            }
            OperationType::Append(_, _) => {
                // Use a reference to the stored bin
                let op = operations::append(&bin_storage[bin_idx]);
                bin_idx += 1;
                op
            }
            OperationType::Prepend(_, _) => {
                // Use a reference to the stored bin
                let op = operations::prepend(&bin_storage[bin_idx]);
                bin_idx += 1;
                op
            }
            OperationType::Delete() => {
                // Use the operations module's delete() function
                operations::delete()
            }
            OperationType::Touch() => {
                // Use the operations module's touch() function
                operations::touch()
            }
            OperationType::ListGet(bin_name, index) => {
                // Use the operations module's list get() function
                use aerospike_core::operations::lists;
                lists::get(bin_name, *index)
            }
            OperationType::ListSize(bin_name) => {
                // Use the operations module's list size() function
                use aerospike_core::operations::lists;
                lists::size(bin_name)
            }
            OperationType::ListJoin(bin_name, separator) => {
                use aerospike_core::operations::lists;
                match separator {
                    Some(sep) => lists::join_by_separator(bin_name, sep),
                    None => lists::join(bin_name),
                }
            }
            OperationType::ListPop(bin_name, index) => {
                // Use the operations module's list pop() function
                use aerospike_core::operations::lists;
                lists::pop(bin_name, *index)
            }
            OperationType::ListClear(bin_name) => {
                // Use the operations module's list clear() function
                use aerospike_core::operations::lists;
                lists::clear(bin_name)
            }
            OperationType::ListGetRange(bin_name, index, count) => {
                // Use the operations module's list get_range() function
                use aerospike_core::operations::lists;
                lists::get_range(bin_name, *index, *count)
            }
            OperationType::ListSet(bin_name, index, _) => {
                // Use the operations module's list set() function with stored value
                use aerospike_core::operations::lists;
                let op = lists::set(bin_name, *index, value_storage[value_idx].clone());
                value_idx += 1;
                op
            }
            OperationType::ListSetWithPolicy(bin_name, index, _, policy) => {
                use aerospike_core::operations::lists;
                let op = lists::set_with_policy(&policy._as, bin_name, *index, value_storage[value_idx].clone());
                value_idx += 1;
                op
            }
            OperationType::ListIncrementByOne(bin_name, index) => {
                use aerospike_core::operations::lists;
                lists::increment_by_one(bin_name, *index)
            }
            OperationType::ListIncrementByOneWithPolicy(bin_name, index, policy) => {
                use aerospike_core::operations::lists;
                lists::increment_by_one_with_policy(&policy._as, bin_name, *index)
            }
            OperationType::ListCreateWithIndex(bin_name, order) => {
                use aerospike_core::operations::lists;
                let core_order: aerospike_core::operations::lists::ListOrderType = order.into();
                lists::create_with_index(bin_name, core_order)
            }
            OperationType::ListSetOrderWithIndex(bin_name, order) => {
                use aerospike_core::operations::lists;
                let core_order: aerospike_core::operations::lists::ListOrderType = order.into();
                lists::set_order_with_index(bin_name, core_order)
            }
            OperationType::ListRemove(bin_name, index) => {
                // Use the operations module's list remove() function
                use aerospike_core::operations::lists;
                lists::remove(bin_name, *index)
            }
            OperationType::ListRemoveRange(bin_name, index, count) => {
                // Use the operations module's list remove_range() function
                use aerospike_core::operations::lists;
                lists::remove_range(bin_name, *index, *count)
            }
            OperationType::ListGetRangeFrom(bin_name, index) => {
                // Use the operations module's list get_range_from() function
                use aerospike_core::operations::lists;
                lists::get_range_from(bin_name, *index)
            }
            OperationType::ListPopRange(bin_name, index, count) => {
                // Use the operations module's list pop_range() function
                use aerospike_core::operations::lists;
                lists::pop_range(bin_name, *index, *count)
            }
            OperationType::ListPopRangeFrom(bin_name, index) => {
                // Use the operations module's list pop_range_from() function
                use aerospike_core::operations::lists;
                lists::pop_range_from(bin_name, *index)
            }
            OperationType::ListRemoveRangeFrom(bin_name, index) => {
                // Use the operations module's list remove_range_from() function
                use aerospike_core::operations::lists;
                lists::remove_range_from(bin_name, *index)
            }
            OperationType::ListTrim(bin_name, index, count) => {
                // Use the operations module's list trim() function
                use aerospike_core::operations::lists;
                lists::trim(bin_name, *index, *count)
            }
            OperationType::ListAppend(bin_name, _, policy) => {
                // Use the operations module's list append() function with stored value and policy
                use aerospike_core::operations::lists;
                let op = lists::append(&policy._as, bin_name, value_storage[value_idx].clone());
                value_idx += 1;
                op
            }
            OperationType::ListAppendItems(bin_name, values, policy) => {
                // Use the operations module's list append_items() function with stored values and policy
                use aerospike_core::operations::lists;
                let values_slice: &[aerospike_core::Value] = &value_storage[value_idx..value_idx + values.len()];
                let op = lists::append_items(&policy._as, bin_name, values_slice.to_vec());
                value_idx += values.len();
                op
            }
            OperationType::ListInsert(bin_name, index, _, policy) => {
                // Use the operations module's list insert() function with stored value and policy
                use aerospike_core::operations::lists;
                let op = lists::insert(&policy._as, bin_name, *index, value_storage[value_idx].clone());
                value_idx += 1;
                op
            }
            OperationType::ListInsertItems(bin_name, index, values, policy) => {
                // Use the operations module's list insert_items() function with stored values and policy
                use aerospike_core::operations::lists;
                let values_slice: &[aerospike_core::Value] = &value_storage[value_idx..value_idx + values.len()];
                let op = lists::insert_items(&policy._as, bin_name, *index, values_slice.to_vec());
                value_idx += values.len();
                op
            }
            OperationType::ListIncrement(bin_name, index, value, policy) => {
                // Use the operations module's list increment() function with policy
                use aerospike_core::operations::lists;
                lists::increment(&policy._as, bin_name, *index, *value)
            }
            OperationType::ListSort(bin_name, flags) => {
                // Use the operations module's list sort() function
                use aerospike_core::operations::lists;
                let core_flags: aerospike_core::operations::lists::ListSortFlags = flags.into();
                lists::sort(bin_name, core_flags)
            }
            OperationType::ListSetOrder(bin_name, order) => {
                // Use the operations module's list set_order() function
                use aerospike_core::operations::lists;
                let core_order: aerospike_core::operations::lists::ListOrderType = order.into();
                lists::set_order(bin_name, core_order)
            }
            OperationType::ListGetByIndex(bin_name, index, return_type) => {
                // Use the operations module's list get_by_index() function with return type
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                lists::get_by_index(bin_name, *index, core_return_type)
            }
            OperationType::ListGetByIndexRange(bin_name, index, count, return_type) => {
                // Use the operations module's list get_by_index_range() or get_by_index_range_count() function
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                match count {
                    Some(c) => lists::get_by_index_range_count(bin_name, *index, *c, core_return_type),
                    None => lists::get_by_index_range(bin_name, *index, core_return_type),
                }
            }
            OperationType::ListGetByRank(bin_name, rank, return_type) => {
                // Use the operations module's list get_by_rank() function with return type
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                lists::get_by_rank(bin_name, *rank, core_return_type)
            }
            OperationType::ListGetByRankRange(bin_name, rank, count, return_type) => {
                // Use the operations module's list get_by_rank_range() or get_by_rank_range_count() function
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                match count {
                    Some(c) => lists::get_by_rank_range_count(bin_name, *rank, *c, core_return_type),
                    None => lists::get_by_rank_range(bin_name, *rank, core_return_type),
                }
            }
            OperationType::ListGetByValue(bin_name, _, return_type) => {
                // Use the operations module's list get_by_value() function with stored value and return type
                use aerospike_core::operations::lists;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = lists::get_by_value(bin_name, value.clone(), core_return_type);
                value_idx += 1;
                op
            }
            OperationType::ListGetByValueRange(bin_name, _, _, return_type) => {
                // Use the operations module's list get_by_value_range() function with stored values and return type
                use aerospike_core::operations::lists;
                let begin = &value_storage[value_idx];
                let end = &value_storage[value_idx + 1];
                let core_return_type = (*return_type).into();
                let op = lists::get_by_value_range(bin_name, begin.clone(), end.clone(), core_return_type);
                value_idx += 2;
                op
            }
            OperationType::ListGetByValueList(bin_name, _, return_type) => {
                // Use the operations module's list get_by_value_list() function with stored list and return type
                use aerospike_core::operations::lists;
                let values = &list_storage[list_idx];
                let core_return_type = (*return_type).into();
                let op = lists::get_by_value_list(bin_name, values.to_vec(), core_return_type);
                list_idx += 1;
                op
            }
            OperationType::ListGetByValueRelativeRankRange(bin_name, _, rank, count, return_type) => {
                // Use the operations module's list get_by_value_relative_rank_range() function
                use aerospike_core::operations::lists;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = match count {
                    Some(c) => lists::get_by_value_relative_rank_range_count(bin_name, value.clone(), *rank, *c, core_return_type),
                    None => lists::get_by_value_relative_rank_range(bin_name, value.clone(), *rank, core_return_type),
                };
                value_idx += 1;
                op
            }
            OperationType::ListRemoveByIndex(bin_name, index, return_type) => {
                // Use the operations module's list remove_by_index() function with return type
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                lists::remove_by_index(bin_name, *index, core_return_type)
            }
            OperationType::ListRemoveByIndexRange(bin_name, index, count, return_type) => {
                // Use the operations module's list remove_by_index_range() or remove_by_index_range_count() function
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                match count {
                    Some(c) => lists::remove_by_index_range_count(bin_name, *index, *c, core_return_type),
                    None => lists::remove_by_index_range(bin_name, *index, core_return_type),
                }
            }
            OperationType::ListRemoveByRank(bin_name, rank, return_type) => {
                // Use the operations module's list remove_by_rank() function with return type
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                lists::remove_by_rank(bin_name, *rank, core_return_type)
            }
            OperationType::ListRemoveByRankRange(bin_name, rank, count, return_type) => {
                // Use the operations module's list remove_by_rank_range() or remove_by_rank_range_count() function
                use aerospike_core::operations::lists;
                let core_return_type = (*return_type).into();
                match count {
                    Some(c) => lists::remove_by_rank_range_count(bin_name, *rank, *c, core_return_type),
                    None => lists::remove_by_rank_range(bin_name, *rank, core_return_type),
                }
            }
            OperationType::ListRemoveByValue(bin_name, _, return_type) => {
                // Use the operations module's list remove_by_value() function with stored value and return type
                use aerospike_core::operations::lists;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = lists::remove_by_value(bin_name, value.clone(), core_return_type);
                value_idx += 1;
                op
            }
            OperationType::ListRemoveByValueList(bin_name, _, return_type) => {
                // Use the operations module's list remove_by_value_list() function with stored list and return type
                use aerospike_core::operations::lists;
                let values = &list_storage[list_idx];
                let core_return_type = (*return_type).into();
                let op = lists::remove_by_value_list(bin_name, values.to_vec(), core_return_type);
                list_idx += 1;
                op
            }
            OperationType::ListRemoveByValueRange(bin_name, _, _, return_type) => {
                // Use the operations module's list remove_by_value_range() function with stored values and return type
                // Note: parameter order is (bin, return_type, begin, end)
                use aerospike_core::operations::lists;
                let begin = &value_storage[value_idx];
                let end = &value_storage[value_idx + 1];
                let core_return_type = (*return_type).into();
                let op = lists::remove_by_value_range(bin_name, core_return_type, begin.clone(), end.clone());
                value_idx += 2;
                op
            }
            OperationType::ListRemoveByValueRelativeRankRange(bin_name, _, rank, count, return_type) => {
                // Use the operations module's list remove_by_value_relative_rank_range() function
                // Note: parameter order is (bin, return_type, value, rank) for no-count version
                use aerospike_core::operations::lists;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = match count {
                    Some(c) => lists::remove_by_value_relative_rank_range_count(bin_name, core_return_type, value.clone(), *rank, *c),
                    None => lists::remove_by_value_relative_rank_range(bin_name, core_return_type, value.clone(), *rank),
                };
                value_idx += 1;
                op
            }
            OperationType::ListCreate(bin_name, order, pad, _persist_index) => {
                // Use the operations module's list create() function
                // Note: Rust core client doesn't support persist_index parameter
                use aerospike_core::operations::lists;
                let core_order: aerospike_core::operations::lists::ListOrderType = order.into();
                lists::create(bin_name, core_order, *pad)
            }
            OperationType::MapSize(bin_name) => {
                // Use the operations module's map size() function
                use aerospike_core::operations::maps;
                maps::size(bin_name)
            }
            OperationType::MapClear(bin_name) => {
                // Use the operations module's map clear() function
                use aerospike_core::operations::maps;
                maps::clear(bin_name)
            }
            OperationType::MapPut(bin_name, _, _, policy) => {
                // Use the operations module's map put() function with stored key, value, and policy
                use aerospike_core::operations::maps;
                let key = &value_storage[value_idx];
                let value = &value_storage[value_idx + 1];
                let op = maps::put(&policy._as, bin_name, key.clone(), value.clone());
                value_idx += 2;
                op
            }
            OperationType::MapPutItems(bin_name, _, policy) => {
                // Use the operations module's map put_items() function with stored items and policy
                use aerospike_core::operations::maps;
                let op = maps::put_items(&policy._as, bin_name, map_storage[map_idx].clone());
                map_idx += 1;
                op
            }
            OperationType::MapIncrementValue(bin_name, _, _value, policy) => {
                // Use the operations module's map increment_value() function with stored key, value, and policy
                use aerospike_core::operations::maps;
                let key = &value_storage[value_idx];
                let incr_value = &value_storage[value_idx + 1];
                let op = maps::increment_value(&policy._as, bin_name, key.clone(), incr_value.clone());
                value_idx += 2;
                op
            }
            OperationType::MapDecrementValue(bin_name, _, _value, policy) => {
                // Use the operations module's map decrement_value() function with stored key, value, and policy
                use aerospike_core::operations::maps;
                let key = &value_storage[value_idx];
                let decr_value = &value_storage[value_idx + 1];
                let op = maps::decrement_value(&policy._as, bin_name, key.clone(), decr_value.clone());
                value_idx += 2;
                op
            }
            OperationType::MapGetByKey(bin_name, _, return_type) => {
                // Use the operations module's map get_by_key() function with stored key and return type
                use aerospike_core::operations::maps;
                let key = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = maps::get_by_key(bin_name, key.clone(), core_return_type);
                value_idx += 1;
                op
            }
            OperationType::MapRemoveByKey(bin_name, _, return_type) => {
                // Use the operations module's map remove_by_key() function with stored key and return type
                use aerospike_core::operations::maps;
                let key = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = maps::remove_by_key(bin_name, key.clone(), core_return_type);
                value_idx += 1;
                op
            }
            OperationType::MapGetByKeyRange(bin_name, _, _, return_type) => {
                // Use the operations module's map get_by_key_range() function with stored keys and return type
                use aerospike_core::operations::maps;
                let begin = &value_storage[value_idx];
                let end = &value_storage[value_idx + 1];
                let core_return_type = (*return_type).into();
                let op = maps::get_by_key_range(bin_name, begin.clone(), end.clone(), core_return_type);
                value_idx += 2;
                op
            }
            OperationType::MapRemoveByKeyRange(bin_name, _, _, return_type) => {
                // Use the operations module's map remove_by_key_range() function with stored keys and return type
                use aerospike_core::operations::maps;
                let begin = &value_storage[value_idx];
                let end = &value_storage[value_idx + 1];
                let core_return_type = (*return_type).into();
                let op = maps::remove_by_key_range(bin_name, begin.clone(), end.clone(), core_return_type);
                value_idx += 2;
                op
            }
            OperationType::MapGetByIndex(bin_name, index, return_type) => {
                // Use the operations module's map get_by_index() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::get_by_index(bin_name, *index, core_return_type)
            }
            OperationType::MapRemoveByIndex(bin_name, index, return_type) => {
                // Use the operations module's map remove_by_index() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::remove_by_index(bin_name, *index, core_return_type)
            }
            OperationType::MapGetByIndexRange(bin_name, index, count, return_type) => {
                // Use the operations module's map get_by_index_range() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::get_by_index_range(bin_name, *index, *count, core_return_type)
            }
            OperationType::MapRemoveByIndexRange(bin_name, index, count, return_type) => {
                // Use the operations module's map remove_by_index_range() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::remove_by_index_range(bin_name, *index, *count, core_return_type)
            }
            OperationType::MapGetByIndexRangeFrom(bin_name, index, return_type) => {
                // Use the operations module's map get_by_index_range_from() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::get_by_index_range_from(bin_name, *index, core_return_type)
            }
            OperationType::MapRemoveByIndexRangeFrom(bin_name, index, return_type) => {
                // Use the operations module's map remove_by_index_range_from() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::remove_by_index_range_from(bin_name, *index, core_return_type)
            }
            OperationType::MapGetByRank(bin_name, rank, return_type) => {
                // Use the operations module's map get_by_rank() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::get_by_rank(bin_name, *rank, core_return_type)
            }
            OperationType::MapRemoveByRank(bin_name, rank, return_type) => {
                // Use the operations module's map remove_by_rank() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::remove_by_rank(bin_name, *rank, core_return_type)
            }
            OperationType::MapGetByRankRange(bin_name, rank, count, return_type) => {
                // Use the operations module's map get_by_rank_range() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::get_by_rank_range(bin_name, *rank, *count, core_return_type)
            }
            OperationType::MapRemoveByRankRange(bin_name, rank, count, return_type) => {
                // Use the operations module's map remove_by_rank_range() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::remove_by_rank_range(bin_name, *rank, *count, core_return_type)
            }
            OperationType::MapGetByRankRangeFrom(bin_name, rank, return_type) => {
                // Use the operations module's map get_by_rank_range_from() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::get_by_rank_range_from(bin_name, *rank, core_return_type)
            }
            OperationType::MapRemoveByRankRangeFrom(bin_name, rank, return_type) => {
                // Use the operations module's map remove_by_rank_range_from() function with return type
                use aerospike_core::operations::maps;
                let core_return_type = (*return_type).into();
                maps::remove_by_rank_range_from(bin_name, *rank, core_return_type)
            }
            OperationType::MapGetByValue(bin_name, _, return_type) => {
                // Use the operations module's map get_by_value() function with stored value and return type
                use aerospike_core::operations::maps;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = maps::get_by_value(bin_name, value.clone(), core_return_type);
                value_idx += 1;
                op
            }
            OperationType::MapRemoveByValue(bin_name, _, return_type) => {
                // Use the operations module's map remove_by_value() function with stored value and return type
                use aerospike_core::operations::maps;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = maps::remove_by_value(bin_name, value.clone(), core_return_type);
                value_idx += 1;
                op
            }
            OperationType::MapGetByValueRange(bin_name, _, _, return_type) => {
                // Use the operations module's map get_by_value_range() function with stored values and return type
                use aerospike_core::operations::maps;
                let begin = &value_storage[value_idx];
                let end = &value_storage[value_idx + 1];
                let core_return_type = (*return_type).into();
                let op = maps::get_by_value_range(bin_name, begin.clone(), end.clone(), core_return_type);
                value_idx += 2;
                op
            }
            OperationType::MapRemoveByValueRange(bin_name, _, _, return_type) => {
                // Use the operations module's map remove_by_value_range() function with stored values and return type
                use aerospike_core::operations::maps;
                let begin = &value_storage[value_idx];
                let end = &value_storage[value_idx + 1];
                let core_return_type = (*return_type).into();
                let op = maps::remove_by_value_range(bin_name, begin.clone(), end.clone(), core_return_type);
                value_idx += 2;
                op
            }
            OperationType::MapGetByKeyList(bin_name, _, return_type) => {
                // Use the operations module's map get_by_key_list() function with stored key list and return type
                use aerospike_core::operations::maps;
                let keys = &list_storage[list_idx];
                let core_return_type = (*return_type).into();
                let op = maps::get_by_key_list(bin_name, keys.to_vec(), core_return_type);
                list_idx += 1;
                op
            }
            OperationType::MapRemoveByKeyList(bin_name, _, return_type) => {
                // Use the operations module's map remove_by_key_list() function with stored key list and return type
                use aerospike_core::operations::maps;
                let keys = &list_storage[list_idx];
                let core_return_type = (*return_type).into();
                let op = maps::remove_by_key_list(bin_name, keys.to_vec(), core_return_type);
                list_idx += 1;
                op
            }
            OperationType::MapGetByValueList(bin_name, _, return_type) => {
                // Use the operations module's map get_by_value_list() function with stored value list and return type
                use aerospike_core::operations::maps;
                let values = &list_storage[list_idx];
                let core_return_type = (*return_type).into();
                let op = maps::get_by_value_list(bin_name, values.to_vec(), core_return_type);
                list_idx += 1;
                op
            }
            OperationType::MapRemoveByValueList(bin_name, _, return_type) => {
                // Use the operations module's map remove_by_value_list() function with stored value list and return type
                use aerospike_core::operations::maps;
                let values = &list_storage[list_idx];
                let core_return_type = (*return_type).into();
                let op = maps::remove_by_value_list(bin_name, values.to_vec(), core_return_type);
                list_idx += 1;
                op
            }
            OperationType::MapSetMapPolicy(bin_name, policy) => {
                // Use the operations module's map set_order() function
                // Note: Rust core client only has set_order, not full setMapPolicy
                // This sets the map order from the policy
                use aerospike_core::operations::maps;
                let core_order = policy._as.order;
                maps::set_order(bin_name, core_order)
            }
            OperationType::MapSetPolicy(bin_name, policy) => {
                use aerospike_core::operations::maps;
                maps::set_policy(&policy._as, bin_name, vec![])
            }
            OperationType::MapCreateWithIndex(bin_name, order) => {
                use aerospike_core::operations::maps;
                let core_order: aerospike_core::operations::maps::MapOrder = order.into();
                maps::create_with_index(bin_name, core_order)
            }
            OperationType::MapGetByKeyRelativeIndexRange(bin_name, _, index, count, return_type) => {
                // Use the operations module's map get_by_key_relative_index_range() function
                use aerospike_core::operations::maps;
                let key = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = match count {
                    Some(c) => maps::get_by_key_relative_index_range_count(bin_name, key.clone(), *index, *c, core_return_type),
                    None => maps::get_by_key_relative_index_range(bin_name, key.clone(), *index, core_return_type),
                };
                value_idx += 1;
                op
            }
            OperationType::MapGetByValueRelativeRankRange(bin_name, _, rank, count, return_type) => {
                // Use the operations module's map get_by_value_relative_rank_range() function
                use aerospike_core::operations::maps;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = match count {
                    Some(c) => maps::get_by_value_relative_rank_range_count(bin_name, value.clone(), *rank, *c, core_return_type),
                    None => maps::get_by_value_relative_rank_range(bin_name, value.clone(), *rank, core_return_type),
                };
                value_idx += 1;
                op
            }
            OperationType::MapRemoveByKeyRelativeIndexRange(bin_name, _, index, count, return_type) => {
                // Use the operations module's map remove_by_key_relative_index_range() function
                use aerospike_core::operations::maps;
                let key = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = match count {
                    Some(c) => maps::remove_by_key_relative_index_range_count(bin_name, key.clone(), *index, *c, core_return_type),
                    None => maps::remove_by_key_relative_index_range(bin_name, key.clone(), *index, core_return_type),
                };
                value_idx += 1;
                op
            }
            OperationType::MapRemoveByValueRelativeRankRange(bin_name, _, rank, count, return_type) => {
                // Use the operations module's map remove_by_value_relative_rank_range() function
                use aerospike_core::operations::maps;
                let value = &value_storage[value_idx];
                let core_return_type = (*return_type).into();
                let op = match count {
                    Some(c) => maps::remove_by_value_relative_rank_range_count(bin_name, value.clone(), *rank, *c, core_return_type),
                    None => maps::remove_by_value_relative_rank_range(bin_name, value.clone(), *rank, core_return_type),
                };
                value_idx += 1;
                op
            }
            OperationType::MapCreate(bin_name, order) => {
                // Use the operations module's map set_order() function
                // Note: Rust core client uses set_order instead of create for maps
                use aerospike_core::operations::maps;
                let core_order: aerospike_core::operations::maps::MapOrder = order.into();
                maps::set_order(bin_name, core_order)
            }
            OperationType::BitResize(bin_name, byte_size, resize_flags, policy) => {
                use aerospike_core::operations::bitwise;
                let flags = resize_flags.map_or(bitwise::BitwiseResizeFlags::DEFAULT, |f| f.into());
                bitwise::resize(bin_name, *byte_size, flags, &policy._as)
            }
            OperationType::BitInsert(bin_name, byte_offset, _, policy) => {
                use aerospike_core::operations::bitwise;
                let value = &value_storage[value_idx];
                let op = bitwise::insert(bin_name, *byte_offset, value.clone(), &policy._as);
                value_idx += 1;
                op
            }
            OperationType::BitRemove(bin_name, byte_offset, byte_size, policy) => {
                use aerospike_core::operations::bitwise;
                bitwise::remove(bin_name, *byte_offset, *byte_size, &policy._as)
            }
            OperationType::BitSet(bin_name, bit_offset, bit_size, _, policy) => {
                use aerospike_core::operations::bitwise;
                let value = &value_storage[value_idx];
                let op = bitwise::set(bin_name, *bit_offset, *bit_size, value.clone(), &policy._as);
                value_idx += 1;
                op
            }
            OperationType::BitOr(bin_name, bit_offset, bit_size, _, policy) => {
                use aerospike_core::operations::bitwise;
                let value = &value_storage[value_idx];
                let op = bitwise::or(bin_name, *bit_offset, *bit_size, value.clone(), &policy._as);
                value_idx += 1;
                op
            }
            OperationType::BitXor(bin_name, bit_offset, bit_size, _, policy) => {
                use aerospike_core::operations::bitwise;
                let value = &value_storage[value_idx];
                let op = bitwise::xor(bin_name, *bit_offset, *bit_size, value.clone(), &policy._as);
                value_idx += 1;
                op
            }
            OperationType::BitAnd(bin_name, bit_offset, bit_size, _, policy) => {
                use aerospike_core::operations::bitwise;
                let value = &value_storage[value_idx];
                let op = bitwise::and(bin_name, *bit_offset, *bit_size, value.clone(), &policy._as);
                value_idx += 1;
                op
            }
            OperationType::BitNot(bin_name, bit_offset, bit_size, policy) => {
                use aerospike_core::operations::bitwise;
                bitwise::not(bin_name, *bit_offset, *bit_size, &policy._as)
            }
            OperationType::BitLShift(bin_name, bit_offset, bit_size, shift, policy) => {
                use aerospike_core::operations::bitwise;
                bitwise::lshift(bin_name, *bit_offset, *bit_size, *shift, &policy._as)
            }
            OperationType::BitRShift(bin_name, bit_offset, bit_size, shift, policy) => {
                use aerospike_core::operations::bitwise;
                bitwise::rshift(bin_name, *bit_offset, *bit_size, *shift, &policy._as)
            }
            OperationType::BitAdd(bin_name, bit_offset, bit_size, value, signed, action, policy) => {
                use aerospike_core::operations::bitwise;
                let core_action: aerospike_core::operations::bitwise::BitwiseOverflowActions = (*action).into();
                bitwise::add(bin_name, *bit_offset, *bit_size, *value, *signed, core_action, &policy._as)
            }
            OperationType::BitSubtract(bin_name, bit_offset, bit_size, value, signed, action, policy) => {
                use aerospike_core::operations::bitwise;
                let core_action: aerospike_core::operations::bitwise::BitwiseOverflowActions = (*action).into();
                bitwise::subtract(bin_name, *bit_offset, *bit_size, *value, *signed, core_action, &policy._as)
            }
            OperationType::BitSetInt(bin_name, bit_offset, bit_size, value, policy) => {
                use aerospike_core::operations::bitwise;
                bitwise::set_int(bin_name, *bit_offset, *bit_size, *value, &policy._as)
            }
            OperationType::BitGet(bin_name, bit_offset, bit_size) => {
                use aerospike_core::operations::bitwise;
                bitwise::get(bin_name, *bit_offset, *bit_size)
            }
            OperationType::BitCount(bin_name, bit_offset, bit_size) => {
                use aerospike_core::operations::bitwise;
                bitwise::count(bin_name, *bit_offset, *bit_size)
            }
            OperationType::BitLScan(bin_name, bit_offset, bit_size, value) => {
                use aerospike_core::operations::bitwise;
                bitwise::lscan(bin_name, *bit_offset, *bit_size, *value)
            }
            OperationType::BitRScan(bin_name, bit_offset, bit_size, value) => {
                use aerospike_core::operations::bitwise;
                bitwise::rscan(bin_name, *bit_offset, *bit_size, *value)
            }
            OperationType::BitGetInt(bin_name, bit_offset, bit_size, signed) => {
                use aerospike_core::operations::bitwise;
                bitwise::get_int(bin_name, *bit_offset, *bit_size, *signed)
            }
            OperationType::BitB64Encode(bin_name, byte_offset, byte_size, invert_size) => {
                use aerospike_core::operations::bitwise;
                match (byte_offset, byte_size) {
                    (Some(offset), Some(size)) => {
                        bitwise::b64_encode_range(bin_name, *offset, *size, *invert_size)
                    }
                    _ => bitwise::b64_encode(bin_name),
                }
            }
            OperationType::HllInit(bin_name, index_bit_count, min_hash_bit_count, flags) => {
                use aerospike_core::operations::hll;
                let policy = aerospike_core::operations::hll::HllPolicy { flags: aerospike_core::operations::hll::HllWriteFlags::from_bits(*flags) };
                hll::init_with_min_hash(&policy, bin_name, *index_bit_count, *min_hash_bit_count)
            }
            OperationType::HllAdd(bin_name, _, index_bit_count, min_hash_bit_count, flags) => {
                use aerospike_core::operations::hll;
                let policy = aerospike_core::operations::hll::HllPolicy { flags: aerospike_core::operations::hll::HllWriteFlags::from_bits(*flags) };
                let values_ref = &hll_value_storage[hll_idx];
                hll_idx += 1;
                hll::add_with_index_and_min_hash(&policy, bin_name, values_ref.to_vec(), *index_bit_count, *min_hash_bit_count)
            }
            OperationType::HllGetCount(bin_name) => {
                use aerospike_core::operations::hll;
                hll::get_count(bin_name)
            }
            OperationType::HllDescribe(bin_name) => {
                use aerospike_core::operations::hll;
                hll::describe(bin_name)
            }
            OperationType::HllRefreshCount(bin_name) => {
                use aerospike_core::operations::hll;
                hll::refresh_count(bin_name)
            }
            OperationType::HllFold(bin_name, index_bit_count) => {
                use aerospike_core::operations::hll;
                hll::fold(bin_name, *index_bit_count)
            }
            OperationType::HllGetUnion(bin_name, _) => {
                use aerospike_core::operations::hll;
                let values_ref = &hll_value_storage[hll_idx];
                hll_idx += 1;
                hll::get_union(bin_name, values_ref.to_vec())
            }
            OperationType::HllGetUnionCount(bin_name, _) => {
                use aerospike_core::operations::hll;
                let values_ref = &hll_value_storage[hll_idx];
                hll_idx += 1;
                hll::get_union_count(bin_name, values_ref.to_vec())
            }
            OperationType::HllGetIntersectCount(bin_name, _) => {
                use aerospike_core::operations::hll;
                let values_ref = &hll_value_storage[hll_idx];
                hll_idx += 1;
                hll::get_intersect_count(bin_name, values_ref.to_vec())
            }
            OperationType::HllGetSimilarity(bin_name, _) => {
                use aerospike_core::operations::hll;
                let values_ref = &hll_value_storage[hll_idx];
                hll_idx += 1;
                hll::get_similarity(bin_name, values_ref.to_vec())
            }
            OperationType::HllSetUnion(bin_name, _, flags) => {
                use aerospike_core::operations::hll;
                let policy = aerospike_core::operations::hll::HllPolicy { flags: aerospike_core::operations::hll::HllWriteFlags::from_bits(*flags) };
                let values_ref = &hll_value_storage[hll_idx];
                hll_idx += 1;
                hll::set_union(&policy, bin_name, values_ref.to_vec())
            }
            OperationType::ExpRead(name, exp, flags) => {
                use aerospike_core::operations::exp::{self, ExpReadFlags};
                let core_flags = ExpReadFlags::from_bits(*flags & ExpReadFlags::EVAL_NO_FAIL.bits());
                exp::read_exp(name, exp._as.clone(), core_flags)
            }
            OperationType::ExpWrite(bin_name, exp, flags) => {
                use aerospike_core::operations::exp::{self, ExpWriteFlags};
                // Only the five defined bits are forwarded, as before.
                let core_flags = ExpWriteFlags::from_bits(*flags & 0x1f);
                exp::write_exp(bin_name, exp._as.clone(), core_flags)
            }
            OperationType::CdtSelectByPath(bin_name, flag, ctx) => {
                use aerospike_core::operations::path::{select_by_path, SelectFlag};
                let core_ctx = crate::cdt::ctx_to_vec(ctx);
                select_by_path(bin_name, SelectFlag::from_bits(*flag), &core_ctx)
            }
            OperationType::CdtModifyByPath(bin_name, flag, exp, ctx) => {
                use aerospike_core::operations::path::{modify_by_path, ModifyFlag};
                let core_ctx = crate::cdt::ctx_to_vec(ctx);
                modify_by_path(bin_name, ModifyFlag::from_bits(*flag), exp._as.clone(), &core_ctx)
            }

            // ----- String ops (server 8.2.0+) -----
            // Performance notes:
            //   - `bin` and string args (`needle` / `pattern` / `value` / etc.) are passed
            //     to rust-core as &str via auto-deref. Zero per-op alloc beyond the args
            //     already owned in the variant.
            //   - `StringPolicy` and rust-core's flag tuples are stack-constructed inline.
            //   - `concat` allocates one Vec<&str> to bridge Vec<String> → &[&str] —
            //     unavoidable for the rust-core API shape (it takes &[&str], not &[String]).
            OperationType::StringStrlen(bin) => {
                aerospike_core::operations::string::strlen(bin)
            }
            OperationType::StringSubstr(bin, start, end) => {
                use aerospike_core::operations::string as str_op;
                match end {
                    Some(e) => str_op::substr(bin, *start, *e),
                    None => str_op::substr_from(bin, *start),
                }
            }
            OperationType::StringCharAt(bin, index) => {
                aerospike_core::operations::string::char_at(bin, *index)
            }
            OperationType::StringFind(bin, needle, occurrence) => {
                use aerospike_core::operations::string as str_op;
                match occurrence {
                    Some(n) => str_op::find_nth(bin, needle, *n),
                    None => str_op::find(bin, needle),
                }
            }
            OperationType::StringContains(bin, needle) => {
                aerospike_core::operations::string::contains(bin, needle)
            }
            OperationType::StringStartsWith(bin, prefix) => {
                aerospike_core::operations::string::starts_with(bin, prefix)
            }
            OperationType::StringEndsWith(bin, suffix) => {
                aerospike_core::operations::string::ends_with(bin, suffix)
            }
            OperationType::StringToInteger(bin) => {
                aerospike_core::operations::string::to_integer(bin)
            }
            OperationType::StringToDouble(bin) => {
                aerospike_core::operations::string::to_double(bin)
            }
            OperationType::StringByteLength(bin) => {
                aerospike_core::operations::string::byte_length(bin)
            }
            OperationType::StringIsNumeric(bin, numeric_type) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::StringNumericType as CoreNT;
                match numeric_type {
                    Some(StringNumericType::Any) => str_op::is_numeric_typed(bin, CoreNT::Any),
                    Some(StringNumericType::Int) => str_op::is_numeric_typed(bin, CoreNT::Int),
                    Some(StringNumericType::Float) => str_op::is_numeric_typed(bin, CoreNT::Float),
                    None => str_op::is_numeric(bin),
                }
            }
            OperationType::StringIsUpper(bin) => {
                aerospike_core::operations::string::is_upper(bin)
            }
            OperationType::StringIsLower(bin) => {
                aerospike_core::operations::string::is_lower(bin)
            }
            OperationType::StringToBlob(bin) => {
                aerospike_core::operations::string::to_blob(bin)
            }
            OperationType::StringSplit(bin, separator) => {
                use aerospike_core::operations::string as str_op;
                match separator {
                    Some(s) => str_op::split_by_separator(bin, s),
                    None => str_op::split(bin),
                }
            }
            OperationType::StringB64Decode(bin) => {
                aerospike_core::operations::string::b64_decode(bin)
            }
            OperationType::StringRegexCompare(bin, pattern, regex_flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::StringRegexFlags as CoreSRF;
                str_op::regex_compare_with_flags(bin, pattern, CoreSRF::from_bits(*regex_flags as i64))
            }

            OperationType::StringInsert(bin, index, value, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::insert(&policy, bin, *index, value)
            }
            OperationType::StringOverwrite(bin, index, value, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::overwrite(&policy, bin, *index, value)
            }
            OperationType::StringConcat(bin, values, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                // Rust-core's concat_list takes &[&str]; build the &str view once here.
                let value_refs: Vec<&str> = values.iter().map(String::as_str).collect();
                str_op::concat_list(&policy, bin, &value_refs)
            }
            OperationType::StringAppend(bin, value, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::append(&policy, bin, value)
            }
            OperationType::StringPrepend(bin, value, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::prepend(&policy, bin, value)
            }
            OperationType::StringSnip(bin, start, end, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                match end {
                    Some(e) => str_op::snip(&policy, bin, *start, *e),
                    None => str_op::snip_from(&policy, bin, *start),
                }
            }
            OperationType::StringReplace(bin, needle, replacement, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::replace(&policy, bin, needle, replacement)
            }
            OperationType::StringReplaceAll(bin, needle, replacement, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::replace_all(&policy, bin, needle, replacement)
            }
            OperationType::StringUpper(bin, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                str_op::upper(&StringPolicy::new(CoreSWF::from_bits(*flags as i64)), bin)
            }
            OperationType::StringLower(bin, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                str_op::lower(&StringPolicy::new(CoreSWF::from_bits(*flags as i64)), bin)
            }
            OperationType::StringCaseFold(bin, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                str_op::case_fold(&StringPolicy::new(CoreSWF::from_bits(*flags as i64)), bin)
            }
            OperationType::StringNormalizeNfc(bin, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                str_op::normalize_nfc(&StringPolicy::new(CoreSWF::from_bits(*flags as i64)), bin)
            }
            OperationType::StringTrimStart(bin, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                str_op::trim_start(&StringPolicy::new(CoreSWF::from_bits(*flags as i64)), bin)
            }
            OperationType::StringTrimEnd(bin, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                str_op::trim_end(&StringPolicy::new(CoreSWF::from_bits(*flags as i64)), bin)
            }
            OperationType::StringTrim(bin, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                str_op::trim(&StringPolicy::new(CoreSWF::from_bits(*flags as i64)), bin)
            }
            OperationType::StringPadStart(bin, target_length, pad_string, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::pad_start(&policy, bin, *target_length, pad_string)
            }
            OperationType::StringPadEnd(bin, target_length, pad_string, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::pad_end(&policy, bin, *target_length, pad_string)
            }
            OperationType::StringRepeat(bin, count, flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
                let policy = StringPolicy::new(CoreSWF::from_bits(*flags as i64));
                str_op::repeat(&policy, bin, *count)
            }
            OperationType::StringRegexReplace(bin, pattern, replacement, regex_flags, write_flags) => {
                use aerospike_core::operations::string as str_op;
                use aerospike_core::operations::string::{
                    StringPolicy, StringRegexFlags as CoreSRF, StringWriteFlags as CoreSWF,
                };
                let policy = StringPolicy::new(CoreSWF::from_bits(*write_flags as i64));
                str_op::regex_replace(&policy, bin, pattern, replacement, CoreSRF::from_bits(*regex_flags as i64))
            }

            OperationType::StringToString(bin) => {
                aerospike_core::operations::string::to_string(bin)
            }
        };

        // Apply context if present
        let final_op = if let Some(ctx) = &op_with_ctx.ctx {
            core_op.context(ctx.as_slice().to_vec())
        } else {
            core_op
        };
        core_ops.push(final_op);
    }

    let has_write = record_batch_ops_have_write(rust_ops);
    Ok((core_ops, has_write))
}
