/* 
Values in Arevel are either a primitive u64 value or an object.
Primitive values are interpreted as doubles for fast math.
If the number is a NaN, we-re-use the unused bits to pack pointers and compact values into it. 

0 00000001010 0000000000000000000000000000000000000000000000000000 = 64
1 11111111111 1000000000000000000000000000000000000000000000000000 = nan
Type (3 bits). Payload 48 bits.
The header type bits are used for:
Type: [False(0)/True(1)] [Pointer(0)/Keyword(1)] [String(0), Object(1)]
This allows fast boolean checks and type checks for strings.

Small strings up to 6 bytes are stored directly as constant symbols without object overhead.
Further optimization - huffman coding of common bytes. Could get us another 50% (Future)

String pointer payloads additionally store length (up to 65k) for fast length access and
length-based inequality check without dereferencing.
Pointer types can have payloads for ranged pointers or direct access into an object's field.

There's a large Keyword space used to store all keywords, functions & user-defined symbols.
Keywords generally evaluate to themselves.
Function symbols encode their arity in their payload (max 64 parameters).
Keywords 0-256 reserved for keywords.
*/

// Data format

// 8 = 1000 in binary
use crate::functions::NativeFn2;
use crate::structs::{Keyword, Module};
use crate::operators::*;
use crate::functions::*;


pub const SIGNALING_NAN: u64 = 0xFFF8_0000_0000_0000;
pub const QUITE_NAN: u64 = 0xFFF0_0000_0000_0000;

pub const LOW32_MASK: u64 = 0x0000_0000_FFFF_FFFF;
pub const HIGH32_MASK: u64 = 0xFFFF_FFFF_0000_0000;

// Clear all type bits. Preserve value bits.
pub const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

pub const VALHEAD_MASK: u64 = 0xFFFF_0000_0000_0000;
// 0 False. 1 True.
pub const VALHEAD_TRUTHY_MASK: u64  = 0xFFF4_0000_0000_0000;
// 0 = Pointer. 1 = Keyword.
pub const VALHEAD_REFTYPE_MASK: u64 = 0xFFF2_0000_0000_0000;
// 0 = String. 1 = Object.
pub const VALHEAD_OBJTYPE_MASK: u64 = 0xFFF1_0000_0000_0000;


// 0-8 Invalid NaN (Do Not Use). 7 valid values total.
// These constant values are based on the bit masks above

// Pointer to error values
pub const VALUE_F_PTR_OBJ: u64 = 0xFFF9_0000_0000_0000;
// Reserved symbol for empty string for bool & str type checking.
pub const VALUE_F_SYM_STR: u64 = 0xFFFA_0000_0000_0000;
// Keyword space for empty values and other "Falsey" symbols.
pub const VALUE_F_SYM_OBJ: u64 = 0xFFFB_0000_0000_0000;
// Pointer to full string objects. 16 bit payload of short length. 
pub const VALUE_T_PTR_STR: u64 = 0xFFFC_0000_0000_0000;
// Pointer to object references or functions. 2 bit payload.
pub const VALUE_T_PTR_OBJ: u64 = 0xFFFD_0000_0000_0000;
// Small strings (up to 6 bytes) encoded directly as payload.
pub const VALUE_T_SYM_STR: u64 = 0xFFFE_0000_0000_0000;
// Keyword space (Keywords, user-defined symbols, etc.)
pub const VALUE_T_SYM_OBJ: u64 = 0xFFFF_0000_0000_0000;


// Falsey/empty value symbols 
// (00-FF reserved for internal symbols for indexing into precedence lookup table)
pub const SYMBOL_FALSE: Keyword = Keyword {
    symbol: 0xFFFB_0000_0000_001A,
    name: "False",
    precedence: None,
    operation: None
};

pub const SYMBOL_NONE: Keyword = Keyword {
    symbol: 0xFFFB_0000_0000_001B,
    name: "None",
    precedence: None,
    operation: None
};

pub const SYMBOL_EMPTY_ARR: u64         = 0xFFFB_0000_0000_0042;


// Empty string - Different header because of the string type bits.
// Set to a value outside the reserved precedence range.
pub const SYMBOL_EMPTY_STR: u64         = 0xFFFA_0000_0000_FFFF;


// Internal sentinel nodes for hash tables. (Unused)
pub const SYMBOL_SENTINEL_EMPTY: u64    = 0xFFFB_0000_0000_004A;
pub const SYMBOL_SENTINEL_DELETED: u64  = 0xFFFB_0000_0000_004B;
pub const SYMBOL_SENTINEL_SENTINEL: u64 = 0xFFFB_0000_0000_004C;


// Truthy value symbols
// Like above, 00-FF reserved for precedence lookup. 
// Note: The reserved keyword numbers should be unique (regardless of truthy/falsey).
pub const SYMBOL_TRUE: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0019,
    name: "True",
    precedence: None,
    operation: None
};

pub const SYMBOL_CALL_FN: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_001C,
    name: "__call__",
    precedence: None,
    operation: None
};




// Constants for supported symbols within expressions.
// Uses the reserved space from 0-255 
// Note: Ensure no ID conflict with the symbols defined in avs
// The IDs represent index into the precedence array.
// Loosely arranged by order of precedence.
pub const SYMBOL_COMMA: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0000,
    name: ",",
    precedence: Some(1),
    operation: None
};

pub const SYMBOL_EQUALS: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0001,
    name: "=",
    precedence: Some(2),
    operation: None

};


pub const SYMBOL_OR: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0002,
    name: "or",
    precedence: Some(3),
    operation: Some(__av_or)
};

pub const SYMBOL_AND: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0003,
    name: "and",
    precedence: Some(4),
    operation: Some(__av_and)
};

pub const SYMBOL_NOT: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0004,
    name: "not",
    precedence: Some(5),
    operation: None    // Not is a unary, so handle it separately
};


pub const SYMBOL_DBL_EQUALS: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0005,
    name: "==",
    precedence: Some(10),
    operation: None
};

pub const SYMBOL_NOT_EQUALS: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0006,
    name: "!=",
    precedence: Some(10),
    operation: None
};


pub const SYMBOL_LT: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0007,
    name: "<",
    precedence: Some(15),
    operation: Some(__av_lt)
};

pub const SYMBOL_LTE: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0008,
    name: "<=",
    precedence: Some(15),
    operation: Some(__av_lte)
};

pub const SYMBOL_GT: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0009,
    name: ">",
    precedence: Some(15),
    operation: Some(__av_gt)
};

pub const SYMBOL_GTE: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_000A,
    name: ">=",
    precedence: Some(15),
    operation: Some(__av_gte)
};


pub const SYMBOL_PLUS: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_000B,
    name: "+",
    precedence: Some(20),
    operation: Some(__av_add)
};

pub const SYMBOL_MINUS: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_000C,
    name: "-",
    precedence: Some(20),
    operation: Some(__av_sub)
};

pub const SYMBOL_MULTIPLY: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_000D,
    name: "*",
    precedence: Some(21),
    operation: Some(__av_mul)
};

pub const SYMBOL_DIVIDE: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_000E,
    name: "/",
    precedence: Some(21),
    operation: Some(__av_div)
};

pub const SYMBOL_MODULO: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_000F,
    name: "%",
    precedence: Some(21),
    operation: None         // TODO
};


pub const SYMBOL_DOT: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0010,
    name: ".",
    precedence: Some(25),
    operation: None         // TODO
};



pub const SYMBOL_OPEN_PAREN: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0011,
    name: "(",
    precedence: Some(30), 
    operation: None
};

pub const SYMBOL_CLOSE_PAREN: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0012,
    name: ")",
    precedence: Some(30),
    operation: None
};


pub const SYMBOL_OPEN_SQBR: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0013,
    name: "[",
    precedence: None,
    operation: None
};

pub const SYMBOL_CLOSE_SQBR: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0014,
    name: "]",
    precedence: None,
    operation: None
};

pub const SYMBOL_OPEN_BRACE: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0015,
    name: "{",
    precedence: None,
    operation: None
};

pub const SYMBOL_CLOSE_BRACE: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0016,
    name: "}",
    precedence: None,
    operation: None
};

pub const SYMBOL_COLON: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0017,
    name: ":",
    precedence: None,
    operation: None
};

pub const SYMBOL_SEMI_COLON: Keyword = Keyword {
    symbol: 0xFFFF_0000_0000_0018,
    name: ";",
    precedence: None,
    operation: None
};



// For built in classes (Use to_symbol to encode these as symbols)
// Object class constants for built-in types.
pub const AV_CLASS_OBJECT: u64 = 0xFFFF_0000_0000_1025;
pub const AV_CLASS_CLASS: u64 = 0xFFFF_0000_0000_1026;
pub const AV_CLASS_FUNCTION: u64 = 0xFFFF_0000_0000_1027;
pub const AV_CLASS_ENVIRONMENT: u64 = 0xFFFF_0000_0000_1028;
pub const AV_CLASS_STRING: u64 = 0xFFFF_0000_0000_1029;


// Reserve up to 65k symbols for standard library usage. (Classes, functions, etc.)

pub const AV_FN_MIN: Module = Module {
    symbol: 0xFFFD_0000_0000_0100,
    name: "min",
    value: NativeFn2::create_atom(__av_min)
};

pub const AV_FN_MAX: Module = Module {
    symbol: 0xFFFD_0000_0000_0101,
    name: "max",
    value: NativeFn2::create_atom(__av_max)
};

pub const AV_FN_ABS: Module = Module {
    symbol: 0xFFFD_0000_0000_0102,
    name: "abs",
    value: NativeFn1::create_atom(__av_abs)
};

pub const AV_FN_CEIL: Module = Module {
    symbol: 0xFFFD_0000_0000_0103,
    name: "ceil",
    value: NativeFn1::create_atom(__av_ceil)
};


pub const AV_FN_FLOOR: Module = Module {
    symbol: 0xFFFD_0000_0000_0104,
    name: "floor",
    value: NativeFn1::create_atom(__av_floor)
};

// TODO: Rename this to "int"?
pub const AV_FN_TRUNC: Module = Module {
    symbol: 0xFFFD_0000_0000_0105,
    name: "truncate",
    value: NativeFn1::create_atom(__av_truncate)
};

pub const AV_FN_ROUND: Module = Module {
    symbol: 0xFFFD_0000_0000_0106,
    name: "round",
    value: NativeFn1::create_atom(__av_round)
};

pub const AV_FN_SQRT: Module = Module {
    symbol: 0xFFFD_0000_0000_0107,
    name: "sqrt",
    value: NativeFn1::create_atom(__av_sqrt)
};


pub const AV_HTTP_REQUEST: u64 = 0xFFFC_0000_0000_1100;
pub const AV_HTTP_PATH: u64 = 0xFFFC_0000_0000_1101;
pub const AV_HTTP_QUERY: u64 = 0xFFFC_0000_0000_1102;

//////////////////////////////////////////////////////////////////////////////////
//                               Error Objects                                  //
//////////////////////////////////////////////////////////////////////////////////
// Top 16 bits = Error code. Bottom 16 = Pointer to obj with metadata.
// Convention: Higher bits for earlier stages. parsing stage -> execution stage.
// Important! Ensure that constants are not re-used!

pub const VALUE_ERR: u64 = 0xFFF9_0000_0000_0000;

pub const PARSE_ERR: u64                    = 0xFFF9_0100_0000_0000;
pub const INTERPRETER_ERR: u64              = 0xFFF9_0010_0000_0000;
pub const RUNTIME_ERR: u64                  = 0xFFF9_0001_0000_0000;

// Parsing errors
pub const PARSE_ERR_UNTERM_STR: u64         = 0xFFF9_0200_0000_0000;
pub const PARSE_ERR_INVALID_FLOAT: u64      = 0xFFF9_0300_0000_0000;
pub const PARSE_ERR_UNKNOWN_TOKEN: u64      = 0xFFF9_0400_0000_0000;
pub const PARSE_ERR_UNEXPECTED_TOKEN: u64   = 0xFFF9_0500_0000_0000;
pub const PARSE_ERR_UNMATCHED_PARENS: u64   = 0xFFF9_0600_0000_0000;
pub const PARSE_ERR_USED_NAME: u64          = 0xFFF9_0700_0000_0000;
pub const PARSE_ERR_UNK_SYMBOL: u64         = 0xFFF9_0800_0000_0000;

// Type checking errors
pub const RUNTIME_ERR_INVALID_TYPE: u64     = 0xFFF9_0001_0000_0000;
// This operation is not allowed with NaN values
pub const RUNTIME_ERR_TYPE_NAN: u64         = 0xFFF9_0002_0000_0000;

// Expected number
pub const RUNTIME_ERR_EXPECTED_NUM: u64     = 0xFFF9_0003_0000_0000;
pub const RUNTIME_ERR_EXPECTED_BOOL: u64    = 0xFFF9_0004_0000_0000;
pub const RUNTIME_ERR_UNK_VAL: u64          = 0xFFF9_0005_0000_0000;
pub const RUNTIME_ERR_CIRCULAR_DEP: u64     = 0xFFF9_0006_0000_0000;
pub const RUNTIME_ERR_MEMORY_ACCESS: u64    = 0xFFF9_0007_0000_0000;

pub const RUNTIME_ERR_EXPECTED_STR: u64     = 0xFFF9_0008_0000_0000;

// Arithmetic errors - 0x00
pub const RUNTIME_ERR_DIV_Z: u64            = 0xFFF9_0009_0000_0000;

// Function errors
pub const RUNTIME_ERR_FN_UNK: u64     = 0xFFF9_000A_0000_0000;
pub const RUNTIME_ERR_FN_ARITY: u64     = 0xFFF9_000B_0000_0000;
pub const RUNTIME_ERR_FN_EXPECTED: u64     = 0xFFF9_000C_0000_0000;

// Note: This must be OR-ed with a symbol header to be a symbol
pub const APP_SYMBOL_START: u64             = 0x0000_0000_0001_0000;