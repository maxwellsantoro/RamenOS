//! Gate-first pure Save data assertions; synthetic tuples establish no authority or IO.
//! Every golden is a literal LE fixture independent of the implementation encoder.
use artifact_store_schema::editor_native_save::{
    DATA_PROTOCOL_VERSION, EditorNativeSaveChromeStateV0 as S, EditorNativeSaveChromeV0 as Chrome,
    EditorNativeSaveClassV0 as C, EditorNativeSaveDataV0 as Data, EditorNativeSaveErrorV0 as E,
    EditorNativeSaveGrantV0 as Grant, EditorNativeSavePhaseV0 as P, INPUT_CAPACITY, MAX_TEXT_LEN,
    SAVE_CATEGORY_MASK, SAVE_CHROME_SCHEMA_VERSION, SAVE_CHROME_STATUS_LEN, SAVE_DATA_LEN,
    SAVE_RECORD_COUNT, SAVE_SCHEMA_VERSION, SURFACE_FORMAT, SURFACE_HEIGHT, SURFACE_STRIDE,
    SURFACE_WIDTH, decode_ipc_reference, decode_shmem_reference, encode_ipc_reference,
    encode_shmem_reference,
};
use artifact_store_schema::editor_preview as read;
use artifact_store_schema::editor_save::{
    EditorActorV0, EditorReceiptOutcomeV0, EditorSaveAllocationV0, EditorSaveBindingV0,
    EditorSaveReceiptV0, EditorSourceBindingV0, EditorTextHeaderV0,
};
use sha2::{Digest, Sha256};
const PREVIEW_GOLDEN: [u8; 464] = [
    0x03, 0x00, 0x00, 0x00, 0xd0, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
    0x00, // 0..16
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x10, 0x0f, 0x0e, 0x0d, 0x0c, 0x0b, 0x0a,
    0x09, // 16..32
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 32..48
    0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x30, 0x2f, 0x2e, 0x2d, 0x2c, 0x2b, 0x2a,
    0x29, // 48..64
    0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31, 0x40, 0x3f, 0x3e, 0x3d, 0x3c, 0x3b, 0x3a,
    0x39, // 64..80
    0x48, 0x47, 0x46, 0x45, 0x44, 0x43, 0x42, 0x41, 0x50, 0x4f, 0x4e, 0x4d, 0x4c, 0x4b, 0x4a,
    0x49, // 80..96
    0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a, 0x59, 0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52,
    0x51, // 96..112
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, // 112..128
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
    0x1f, // 128..144
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
    0x2f, // 144..160
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e,
    0x3f, // 160..176
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
    0x24, // 176..192
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8,
    0x55, // 192..208
    0x00, 0x10, 0x00, 0x00, 0x80, 0x02, 0x00, 0x00, 0xe0, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00,
    0x00, // 208..224
    0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 224..240
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 240..256
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52,
    0x51, // 256..272
    0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a, 0x59, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, // 272..288
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 288..304
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 304..320
    0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52, 0x51, 0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a,
    0x59, // 320..336
    0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 336..352
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 352..368
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52,
    0x51, // 368..384
    0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a, 0x59, 0x03, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00,
    0x00, // 384..400
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 400..416
    0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x30, 0x2f, 0x2e, 0x2d, 0x2c, 0x2b, 0x2a,
    0x29, // 416..432
    0x68, 0x67, 0x66, 0x65, 0x64, 0x63, 0x62, 0x61, 0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a,
    0x59, // 432..448
    0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 448..464
];
const ACTIVE_GOLDEN: [u8; 464] = [
    0x03, 0x00, 0x00, 0x00, 0xd0, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
    0x00, // 0..16
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x10, 0x0f, 0x0e, 0x0d, 0x0c, 0x0b, 0x0a,
    0x09, // 16..32
    0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0x20, 0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a,
    0x19, // 32..48
    0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x30, 0x2f, 0x2e, 0x2d, 0x2c, 0x2b, 0x2a,
    0x29, // 48..64
    0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31, 0x40, 0x3f, 0x3e, 0x3d, 0x3c, 0x3b, 0x3a,
    0x39, // 64..80
    0x48, 0x47, 0x46, 0x45, 0x44, 0x43, 0x42, 0x41, 0x50, 0x4f, 0x4e, 0x4d, 0x4c, 0x4b, 0x4a,
    0x49, // 80..96
    0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a, 0x59, 0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52,
    0x51, // 96..112
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, // 112..128
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
    0x1f, // 128..144
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
    0x2f, // 144..160
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e,
    0x3f, // 160..176
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
    0x24, // 176..192
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8,
    0x55, // 192..208
    0x00, 0x10, 0x00, 0x00, 0x80, 0x02, 0x00, 0x00, 0xe0, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00,
    0x00, // 208..224
    0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 224..240
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12,
    0x11, // 240..256
    0x20, 0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a, 0x19, 0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52,
    0x51, // 256..272
    0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a, 0x59, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, // 272..288
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00,
    0x01, // 288..304
    0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0x20, 0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a,
    0x19, // 304..320
    0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52, 0x51, 0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a,
    0x59, // 320..336
    0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 336..352
    0xef, 0xcd, 0xab, 0x89, 0x02, 0x00, 0x00, 0x01, 0x7a, 0x79, 0x78, 0x77, 0x76, 0x75, 0x74,
    0x73, // 352..368
    0x8a, 0x89, 0x88, 0x87, 0x86, 0x85, 0x84, 0x83, 0x58, 0x57, 0x56, 0x55, 0x54, 0x53, 0x52,
    0x51, // 368..384
    0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a, 0x59, 0x03, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00,
    0x00, // 384..400
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
    0x01, // 400..416
    0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x30, 0x2f, 0x2e, 0x2d, 0x2c, 0x2b, 0x2a,
    0x29, // 416..432
    0x68, 0x67, 0x66, 0x65, 0x64, 0x63, 0x62, 0x61, 0x60, 0x5f, 0x5e, 0x5d, 0x5c, 0x5b, 0x5a,
    0x59, // 432..448
    0x04, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 448..464
];
const NO_SAVE_GOLDEN: [u8; 248] = [
    0x02, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 0..16
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 16..32
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 32..48
    0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 48..64
    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 64..80
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 80..96
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 96..112
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 112..128
    0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 128..144
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 144..160
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 160..176
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 176..192
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 192..208
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 208..224
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 224..240
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 240..248
];
const UNKNOWN_GOLDEN: [u8; 248] = [
    0x02, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 0..16
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 16..32
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 32..48
    0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 48..64
    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 64..80
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 80..96
    0x04, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x02, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 96..112
    0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 112..128
    0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 128..144
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, // 144..160
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, // 160..176
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, // 176..192
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, // 192..208
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 208..224
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 224..240
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 240..248
];
const COMMITTED_GOLDEN: [u8; 248] = [
    0x02, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 0..16
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 16..32
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 32..48
    0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 48..64
    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 64..80
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 80..96
    0x04, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x02, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 96..112
    0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 112..128
    0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 128..144
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, // 144..160
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, // 160..176
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, // 176..192
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, // 192..208
    0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
    0x33, // 208..224
    0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
    0x33, // 224..240
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 240..248
];
const NONCOMMIT_GOLDEN: [u8; 248] = [
    0x02, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 0..16
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 16..32
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 32..48
    0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 48..64
    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 64..80
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 80..96
    0x04, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x02, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 96..112
    0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 112..128
    0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 128..144
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, // 144..160
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, // 160..176
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, // 176..192
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, // 192..208
    0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
    0x33, // 208..224
    0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
    0x33, // 224..240
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 240..248
];
const UNAVAILABLE_GOLDEN: [u8; 248] = [
    0x02, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 0..16
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 16..32
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 32..48
    0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 48..64
    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 64..80
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 80..96
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 96..112
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 112..128
    0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 128..144
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 144..160
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 160..176
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 176..192
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 192..208
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 208..224
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 224..240
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 240..248
];
const EMPTY_HASH: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];
const SESSION: u64 = 0x0102_0304_0506_0708;
const SESSION_GEN: u64 = 0x090a_0b0c_0d0e_0f10;
const INSTANCE: u64 = 0x1112_1314_1516_1718;
const INSTANCE_GEN: u64 = 0x191a_1b1c_1d1e_1f20;
const OBJECT: u64 = 0x2122_2324_2526_2728;
const OBJECT_GEN: u64 = 0x292a_2b2c_2d2e_2f30;
const DESKTOP_EPOCH: u64 = 0x5152_5354_5556_5758;
const STORE_EPOCH: u64 = 0x6162_6364_6566_6768;
const EXPIRES: u64 = 0x595a_5b5c_5d5e_5f60;

fn fixture(phase: P) -> Data {
    let active = phase == P::Active;
    let (instance, generation, surface, surface_generation) = if active {
        (
            INSTANCE,
            INSTANCE_GEN,
            0x7374_7576_7778_797a,
            0x8384_8586_8788_898a,
        )
    } else {
        (0, 0, 0, 0)
    };
    let resources = [
        (instance, generation),
        (instance, generation),
        (surface, surface_generation),
        (OBJECT, OBJECT_GEN),
    ];
    let handles = [
        0x0100_0000_0000_0001,
        0x0100_ffff_ffff_ffff,
        0x0100_0002_89ab_cdef,
        0x0100_0004_0000_0007,
    ];
    let classes = [C::SelfStatus, C::FocusRead, C::Surface, C::StoreArtifact];
    let rights = [1, 1, 15, 7];
    Data {
        phase,
        schema_version: 3,
        total_len: 464,
        category_mask: 63,
        record_count: 4,
        session_id: SESSION,
        session_generation: SESSION_GEN,
        instance_id: instance,
        instance_generation: generation,
        selected_object_id: OBJECT,
        selected_object_generation: OBJECT_GEN,
        selected_revision: 0x3132_3334_3536_3738,
        policy_revision: 0x393a_3b3c_3d3e_3f40,
        plan_id: 0x4142_4344_4546_4748,
        preview_revision: 0x494a_4b4c_4d4e_4f50,
        expires_at_ms: EXPIRES,
        desktop_service_epoch: DESKTOP_EPOCH,
        application_hash: core::array::from_fn(|i| i as u8),
        manifest_hash: core::array::from_fn(|i| (i + 32) as u8),
        selected_content_hash: EMPTY_HASH,
        max_text_len: 4096,
        surface_width: 640,
        surface_height: 480,
        surface_stride: 2560,
        surface_format: 1,
        input_capacity: 64,
        protocol_version: 1,
        reserved: 0,
        records: core::array::from_fn(|i| Grant {
            handle: if active { handles[i] } else { 0 },
            resource_id: resources[i].0,
            resource_generation: resources[i].1,
            service_epoch: if i < 3 { DESKTOP_EPOCH } else { STORE_EPOCH },
            expires_at_ms: EXPIRES,
            class: classes[i],
            rights: rights[i],
            protocol_version: 1,
            reserved: 0,
        }),
    }
}
fn chrome(state: S) -> Chrome {
    let operation = matches!(state, S::Unknown | S::Committed | S::DefinitiveNoncommit);
    Chrome {
        schema_version: 2,
        total_len: 248,
        state,
        reserved: 0,
        actor: EditorActorV0 {
            owner_id: 1,
            session_id: 2,
            session_generation: 3,
            instance_id: 4,
            instance_generation: 5,
        },
        selected_object_id: 6,
        selected_object_generation: 7,
        current_store_epoch: 9,
        original_backend_epoch: if operation { 8 } else { 0 },
        operation_id: if operation { 10 } else { 0 },
        source_handle: if operation { 0x0200_0003_0000_0004 } else { 0 },
        source_generation: if operation { 4 } else { 0 },
        expected_revision: if operation { 11 } else { 0 },
        result_revision: if state == S::Committed { 12 } else { 0 },
        attachment_version: 13,
        status_version: 14,
        source_content_hash: if operation { [0x11; 32] } else { [0; 32] },
        expected_content_hash: if operation { [0x22; 32] } else { [0; 32] },
        receipt_sha256: if matches!(state, S::Committed | S::DefinitiveNoncommit) {
            [0x33; 32]
        } else {
            [0; 32]
        },
        source_body_len: if operation { 3 } else { 0 },
        tail_reserved: 0,
    }
}
fn binding() -> EditorSaveBindingV0 {
    EditorSaveBindingV0 {
        schema_version: 1,
        allocation: EditorSaveAllocationV0 {
            schema_version: 1,
            actor: chrome(S::Unknown).actor,
            selected_object_id: 6,
            selected_generation: 7,
            operation_id: 10,
            expected_revision: 11,
            expected_content_hash: [0x22; 32],
        },
        source: EditorSourceBindingV0 {
            source_object_id: 0x0200_0003_0000_0004,
            source_generation: 4,
            byte_len: 3,
            content_hash: [0x11; 32],
        },
    }
}
fn receipt(state: S) -> EditorSaveReceiptV0 {
    EditorSaveReceiptV0 {
        schema_version: 1,
        total_len: 176,
        outcome: if state == S::Committed {
            EditorReceiptOutcomeV0::Committed
        } else {
            EditorReceiptOutcomeV0::DefinitiveNoncommit
        },
        reserved: 0,
        owner_id: 1,
        session_id: 2,
        session_generation: 3,
        original_instance_id: 4,
        original_instance_generation: 5,
        selected_object_id: 6,
        selected_generation: 7,
        operation_id: 10,
        expected_revision: 11,
        result_revision: if state == S::Committed { 12 } else { 0 },
        backend_service_epoch: 8,
        committed_at_ms: 0,
        expected_content_hash: [0x22; 32],
        source_content_hash: [0x11; 32],
    }
}
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn with_receipt(state: S) -> (Chrome, [u8; 176]) {
    let bytes = receipt(state)
        .encode_le()
        .expect("existing pure receipt fixture");
    let mut c = chrome(state);
    c.receipt_sha256 = digest(&bytes);
    (c, bytes)
}
fn word32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn word64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn deny_data(raw: &[u8], expected: E) {
    assert_eq!(Data::decode_le(raw), Err(expected));
    assert_eq!(Data::decode_le_for_phase(raw, P::Preview), Err(expected));
    assert_eq!(Data::decode_le_for_phase(raw, P::Active), Err(expected));
}
fn deny_typed(data: &Data, expected: E) {
    assert_eq!(data.validate(), Err(expected));
    assert_eq!(data.encode_le(), Err(expected));
    assert_eq!(data.validate_for_phase(P::Preview), Err(expected));
    assert_eq!(data.validate_for_phase(P::Active), Err(expected));
}
fn deny_chrome(raw: &[u8], expected: E) {
    assert_eq!(Chrome::decode_le(raw), Err(expected));
}
fn deny_chrome_typed(c: &Chrome, expected: E) {
    assert_eq!(c.validate(), Err(expected));
    assert_eq!(c.encode_le(), Err(expected));
}
fn positive_data() {
    for (phase, golden) in [(P::Preview, PREVIEW_GOLDEN), (P::Active, ACTIVE_GOLDEN)] {
        let d = fixture(phase);
        assert_eq!(d.validate(), Ok(()));
        assert_eq!(d.validate_for_phase(phase), Ok(()));
        assert_eq!(d.encode_le(), Ok(golden));
        assert_eq!(Data::decode_le(&golden), Ok(d));
        assert_eq!(Data::decode_le_for_phase(&golden, phase), Ok(d));
    }
}
fn chrome_goldens() -> [(S, [u8; 248]); 5] {
    [
        (S::NoSave, NO_SAVE_GOLDEN),
        (S::Unknown, UNKNOWN_GOLDEN),
        (S::Committed, COMMITTED_GOLDEN),
        (S::DefinitiveNoncommit, NONCOMMIT_GOLDEN),
        (S::Unavailable, UNAVAILABLE_GOLDEN),
    ]
}

#[test]
fn native_save_grants_literal_le_464() {
    assert_eq!(
        (
            SAVE_SCHEMA_VERSION,
            SAVE_DATA_LEN,
            SAVE_CATEGORY_MASK,
            SAVE_RECORD_COUNT
        ),
        (3, 464, 63, 4)
    );
    assert_eq!(
        (SAVE_CHROME_SCHEMA_VERSION, SAVE_CHROME_STATUS_LEN),
        (2, 248)
    );
    assert_eq!(
        (MAX_TEXT_LEN, SURFACE_WIDTH, SURFACE_HEIGHT, SURFACE_STRIDE),
        (4096, 640, 480, 2560)
    );
    assert_eq!(
        (SURFACE_FORMAT, INPUT_CAPACITY, DATA_PROTOCOL_VERSION),
        (1, 64, 1)
    );
    fn copy_contract<T: Copy + Clone + core::fmt::Debug + PartialEq + Eq>() {}
    copy_contract::<E>();
    copy_contract::<P>();
    copy_contract::<C>();
    copy_contract::<Grant>();
    copy_contract::<Data>();
    copy_contract::<S>();
    assert_eq!((P::Preview as u32, P::Active as u32), (0, 1));
    assert_eq!(
        (
            C::SelfStatus as u32,
            C::FocusRead as u32,
            C::Surface as u32,
            C::StoreArtifact as u32
        ),
        (1, 2, 3, 4)
    );
    positive_data();
    for phase in [P::Preview, P::Active] {
        let mut d = fixture(phase);
        d.session_id = u64::MAX;
        d.session_generation = u64::MAX;
        d.selected_object_id = u64::MAX;
        d.selected_object_generation = u64::MAX;
        d.selected_revision = u64::MAX;
        d.policy_revision = u64::MAX;
        d.plan_id = u64::MAX;
        d.preview_revision = u64::MAX;
        d.expires_at_ms = u64::MAX;
        d.desktop_service_epoch = u64::MAX;
        d.records[3].resource_id = u64::MAX;
        d.records[3].resource_generation = u64::MAX;
        d.records[3].service_epoch = u64::MAX;
        for r in &mut d.records {
            r.expires_at_ms = u64::MAX;
        }
        for r in &mut d.records[..3] {
            r.service_epoch = u64::MAX;
        }
        if phase == P::Active {
            d.instance_id = u64::MAX;
            d.instance_generation = u64::MAX;
            for r in &mut d.records[..3] {
                r.resource_id = u64::MAX;
                r.resource_generation = u64::MAX;
            }
        }
        assert_eq!(d.validate(), Ok(()));
        let raw = d.encode_le().unwrap();
        assert_eq!(Data::decode_le(&raw), Ok(d));
        assert_eq!(&raw[16..32], &[0xff; 16]);
        assert_eq!(&raw[48..112], &[0xff; 64]);
        d.application_hash = [0; 32];
        d.manifest_hash = [0; 32];
        d.selected_content_hash = [0; 32];
        assert_eq!(d.validate(), Ok(()));
    }
}

#[test]
fn native_save_grants_phase_versions_and_rights() {
    positive_data();
    for (phase, golden) in [(P::Preview, PREVIEW_GOLDEN), (P::Active, ACTIVE_GOLDEN)] {
        for len in 0..464 {
            deny_data(&golden[..len], E::Length);
        }
        let mut tail = golden.to_vec();
        tail.push(0);
        deny_data(&tail, E::Length);
        for n in [0, 463, 465, u32::MAX] {
            let mut raw = golden;
            word32(&mut raw, 4, n);
            deny_data(&raw, E::Length);
        }
        for v in [0, 1, 2, 4, u32::MAX] {
            let mut raw = golden;
            word32(&mut raw, 0, v);
            deny_data(&raw, E::Version);
        }
        for o in [236, 292, 348, 404, 460] {
            let mut raw = golden;
            word32(&mut raw, o, 1);
            deny_data(&raw, E::Reserved);
        }
        for (o, v) in [
            (8, 62),
            (12, 3),
            (208, 4095),
            (212, 641),
            (216, 481),
            (220, 2559),
            (224, 2),
            (228, 63),
            (232, 2),
        ] {
            let mut raw = golden;
            word32(&mut raw, o, v);
            deny_data(&raw, E::Constants);
        }
        for i in 0..4 {
            for (rel, v) in [
                (40, 0),
                (40, 5),
                (40, if i == 0 { 2 } else { 1 }),
                (44, 0),
                (44, if i == 3 { 1 } else { 7 }),
                (48, 0),
                (48, 2),
            ] {
                let mut raw = golden;
                word32(&mut raw, 240 + i * 56 + rel, v);
                deny_data(&raw, E::Constants);
            }
        }
        for (id, gen) in [(0, 1), (1, 0)] {
            let mut raw = golden;
            word64(&mut raw, 32, id);
            word64(&mut raw, 40, gen);
            deny_data(&raw, E::Phase);
        }
        let other = if phase == P::Active {
            P::Preview
        } else {
            P::Active
        };
        assert_eq!(Data::decode_le_for_phase(&golden, other), Err(E::Phase));
        assert_eq!(fixture(phase).validate_for_phase(other), Err(E::Phase));
        let mut typed = fixture(phase);
        typed.phase = other;
        deny_typed(&typed, E::Phase);
    }
    for i in 0..12 {
        let mut d = fixture(P::Active);
        match i {
            0 => d.total_len = 463,
            1 => d.schema_version = 2,
            2 => d.reserved = 1,
            3 => d.category_mask = 62,
            4 => d.record_count = 3,
            5 => d.max_text_len = 4095,
            6 => d.surface_width = 639,
            7 => d.surface_height = 479,
            8 => d.surface_stride = 2559,
            9 => d.surface_format = 2,
            10 => d.input_capacity = 63,
            11 => d.protocol_version = 2,
            _ => unreachable!(),
        }
        deny_typed(
            &d,
            match i {
                0 => E::Length,
                1 => E::Version,
                2 => E::Reserved,
                _ => E::Constants,
            },
        );
    }
    for i in 0..4 {
        for f in 0..4 {
            let mut d = fixture(P::Active);
            match f {
                0 => d.records[i].class = if i == 0 { C::FocusRead } else { C::SelfStatus },
                1 => d.records[i].rights = 0,
                2 => d.records[i].protocol_version = 2,
                3 => d.records[i].reserved = 1,
                _ => unreachable!(),
            }
            deny_typed(&d, if f == 3 { E::Reserved } else { E::Constants });
        }
    }
    // Successive repairs expose each earlier priority, including unknown raw class.
    let mut raw = ACTIVE_GOLDEN;
    word32(&mut raw, 4, 0);
    word32(&mut raw, 0, 2);
    word32(&mut raw, 460, 1);
    word32(&mut raw, 280, 99);
    word64(&mut raw, 40, 0);
    deny_data(&raw, E::Length);
    word32(&mut raw, 4, 464);
    deny_data(&raw, E::Version);
    word32(&mut raw, 0, 3);
    deny_data(&raw, E::Reserved);
    word32(&mut raw, 460, 0);
    deny_data(&raw, E::Constants);
    word32(&mut raw, 280, 1);
    deny_data(&raw, E::Phase);
    let mut typed = fixture(P::Active);
    typed.schema_version = 2;
    deny_typed(&typed, E::Version);
    typed = fixture(P::Active);
    typed.records[3].rights = 1;
    deny_typed(&typed, E::Constants);
}

#[test]
fn native_save_grants_handles_resources_epochs_expiry() {
    positive_data();
    for (index, generation, raw) in [
        (0, 1, 0x0100_0000_0000_0001),
        (65535, u32::MAX as u64, 0x0100_ffff_ffff_ffff),
    ] {
        assert_eq!(encode_ipc_reference(index, generation), Ok(raw));
        assert_eq!(decode_ipc_reference(raw), Ok((index, generation)));
    }
    for (index, generation, raw) in [
        (1, 1, 0x0200_0001_0000_0001),
        (65535, u32::MAX as u64, 0x0200_ffff_ffff_ffff),
    ] {
        assert_eq!(encode_shmem_reference(index, generation), Ok(raw));
        assert_eq!(decode_shmem_reference(raw), Ok((index, generation)));
    }
    for (slot, gen) in [
        (65536, 1),
        (u32::MAX, 1),
        (1, 0),
        (1, 1u64 << 32),
        (1, u64::MAX),
    ] {
        assert_eq!(encode_ipc_reference(slot, gen), Err(E::Handle));
        assert_eq!(encode_shmem_reference(slot, gen), Err(E::Handle));
    }
    assert_eq!(encode_shmem_reference(0, 1), Err(E::Handle));
    for raw in [
        0,
        1,
        0x0101_0001_0000_0001,
        0x0100_0001_0000_0000,
        0x0200_0001_0000_0001,
        0xff00_0001_0000_0001,
    ] {
        assert_eq!(decode_ipc_reference(raw), Err(E::Handle));
    }
    for raw in [
        0,
        1,
        0x0201_0001_0000_0001,
        0x0200_0000_0000_0001,
        0x0200_0001_0000_0000,
        0x0100_0001_0000_0001,
    ] {
        assert_eq!(decode_shmem_reference(raw), Err(E::Handle));
    }
    for (index, golden) in [PREVIEW_GOLDEN, ACTIVE_GOLDEN].into_iter().enumerate() {
        let phase = if index == 0 { P::Preview } else { P::Active };
        for o in [16, 24, 48, 56, 64, 72, 80, 88, 96, 104] {
            let mut raw = golden;
            word64(&mut raw, o, 0);
            deny_data(&raw, E::Identity);
        }
        for i in 0..4 {
            let mut raw = golden;
            word64(
                &mut raw,
                240 + i * 56,
                if phase == P::Preview {
                    0x0100_0000_0000_0001
                } else {
                    0
                },
            );
            deny_data(&raw, E::Handle);
            for rel in [8, 16] {
                if i != 2 || phase == P::Preview {
                    let mut raw = golden;
                    word64(&mut raw, 240 + i * 56 + rel, 1);
                    deny_data(&raw, E::Resource);
                } else {
                    let mut raw = golden;
                    word64(&mut raw, 240 + i * 56 + rel, 0);
                    deny_data(&raw, E::Resource);
                }
            }
            let mut raw = golden;
            word64(&mut raw, 240 + i * 56 + 24, if i == 3 { 0 } else { 1 });
            deny_data(&raw, E::Epoch);
            let mut raw = golden;
            word64(&mut raw, 240 + i * 56 + 32, EXPIRES - 1);
            deny_data(&raw, E::Expiry);
        }
    }
    for i in 0..4 {
        for bad in [
            0x0200_0001_0000_0001,
            0x0101_0001_0000_0001,
            0x0100_0001_0000_0000,
        ] {
            let mut raw = ACTIVE_GOLDEN;
            word64(&mut raw, 240 + i * 56, bad);
            deny_data(&raw, E::Handle);
        }
        let mut raw = ACTIVE_GOLDEN;
        word64(&mut raw, 240 + i * 56, 0x0100_0000_0000_0001);
        if i == 0 {
            assert_eq!(Data::decode_le(&raw), Ok(fixture(P::Active)));
        } else {
            deny_data(&raw, E::Handle);
        }
    }
    // IPC slot0 is canonical; choose a distinct generation to keep pairwise uniqueness.
    let mut d = fixture(P::Active);
    d.records[3].handle = 0x0100_0000_0000_0009;
    assert_eq!(d.validate(), Ok(()));
    // Pairwise uniqueness includes pairs that do not involve record0.
    for i in 0..4 {
        for j in i + 1..4 {
            let mut d = fixture(P::Active);
            d.records[j].handle = d.records[i].handle;
            deny_typed(&d, E::Handle);
            let mut raw = ACTIVE_GOLDEN;
            word64(&mut raw, 240 + j * 56, fixture(P::Active).records[i].handle);
            deny_data(&raw, E::Handle);
        }
    }
    for phase in [P::Preview, P::Active] {
        for i in 0..4 {
            for f in 0..5 {
                let mut d = fixture(phase);
                match f {
                    0 => {
                        d.records[i].handle = if phase == P::Preview {
                            0x0100_0000_0000_0001
                        } else {
                            0
                        }
                    }
                    1 => {
                        d.records[i].resource_id = if i == 2 && phase == P::Active { 0 } else { 1 }
                    }
                    2 => {
                        d.records[i].resource_generation =
                            if i == 2 && phase == P::Active { 0 } else { 1 }
                    }
                    3 => d.records[i].service_epoch = 0,
                    4 => d.records[i].expires_at_ms = 0,
                    _ => unreachable!(),
                }
                deny_typed(
                    &d,
                    match f {
                        0 => E::Handle,
                        1 | 2 => E::Resource,
                        3 => E::Epoch,
                        _ => E::Expiry,
                    },
                );
            }
        }
    }
    // Remaining cross-rule precedence after constants/phase.
    let mut raw = ACTIVE_GOLDEN;
    word64(&mut raw, 16, 0);
    word64(&mut raw, 240, 0);
    word64(&mut raw, 248, 0);
    word64(&mut raw, 264, 0);
    word64(&mut raw, 272, 0);
    deny_data(&raw, E::Identity);
    word64(&mut raw, 16, SESSION);
    deny_data(&raw, E::Handle);
    word64(&mut raw, 240, 0x0100_0000_0000_0001);
    deny_data(&raw, E::Resource);
    word64(&mut raw, 248, INSTANCE);
    deny_data(&raw, E::Epoch);
    word64(&mut raw, 264, DESKTOP_EPOCH);
    deny_data(&raw, E::Expiry);
}

#[test]
fn native_save_chrome_literal_le_248() {
    for (state, golden) in chrome_goldens() {
        let c = chrome(state);
        assert_eq!(c.validate(), Ok(()));
        assert_eq!(c.encode_le(), Ok(golden));
        assert_eq!(Chrome::decode_le(&golden), Ok(c));
        for len in 0..248 {
            deny_chrome(&golden[..len], E::Length);
        }
        let mut tail = golden.to_vec();
        tail.push(0);
        deny_chrome(&tail, E::Length);
        for n in [0, 247, 249, u32::MAX] {
            let mut raw = golden;
            word32(&mut raw, 4, n);
            deny_chrome(&raw, E::Length);
        }
        for version in [0, 1, 3, u32::MAX] {
            let mut raw = golden;
            word32(&mut raw, 0, version);
            deny_chrome(&raw, E::Version);
        }
        for o in [12, 244] {
            let mut raw = golden;
            word32(&mut raw, o, 1);
            deny_chrome(&raw, E::Reserved);
        }
        for invalid in [5, u32::MAX] {
            let mut raw = golden;
            word32(&mut raw, 8, invalid);
            deny_chrome(&raw, E::State);
        }
        for o in [16, 24, 32, 40, 48, 56, 64, 72, 128, 136] {
            let mut raw = golden;
            word64(&mut raw, o, 0);
            deny_chrome(&raw, E::Identity);
        }
        let mut wide = chrome(state);
        wide.actor = EditorActorV0 {
            owner_id: u64::MAX,
            session_id: u64::MAX,
            session_generation: u64::MAX,
            instance_id: u64::MAX,
            instance_generation: u64::MAX,
        };
        wide.selected_object_id = u64::MAX;
        wide.selected_object_generation = u64::MAX;
        wide.current_store_epoch = u64::MAX;
        wide.attachment_version = u64::MAX;
        wide.status_version = u64::MAX;
        if matches!(state, S::Unknown | S::Committed | S::DefinitiveNoncommit) {
            wide.original_backend_epoch = u64::MAX;
            wide.operation_id = u64::MAX;
        }
        assert_eq!(wide.validate(), Ok(()));
        let raw = wide.encode_le().unwrap();
        assert_eq!(Chrome::decode_le(&raw), Ok(wide));
        assert_eq!(&raw[16..80], &[0xff; 64]);
        assert_eq!(&raw[128..144], &[0xff; 16]);
    }
    let mut raw = UNKNOWN_GOLDEN;
    word32(&mut raw, 4, 0);
    word32(&mut raw, 0, 1);
    word32(&mut raw, 244, 1);
    word32(&mut raw, 8, 99);
    word64(&mut raw, 16, 0);
    deny_chrome(&raw, E::Length);
    word32(&mut raw, 4, 248);
    deny_chrome(&raw, E::Version);
    word32(&mut raw, 0, 2);
    deny_chrome(&raw, E::Reserved);
    word32(&mut raw, 244, 0);
    deny_chrome(&raw, E::State);
    word32(&mut raw, 8, 1);
    deny_chrome(&raw, E::Identity);
    let mut c = chrome(S::Unknown);
    c.actor.instance_id = 0;
    deny_chrome_typed(&c, E::Identity);
    c = chrome(S::NoSave);
    c.total_len = 247;
    deny_chrome_typed(&c, E::Length);
    c = chrome(S::NoSave);
    c.schema_version = 1;
    deny_chrome_typed(&c, E::Version);
    c = chrome(S::NoSave);
    c.tail_reserved = 1;
    deny_chrome_typed(&c, E::Reserved);
}

#[test]
fn native_save_chrome_idle_and_unknown_matrix() {
    for (index, golden) in [NO_SAVE_GOLDEN, UNAVAILABLE_GOLDEN].into_iter().enumerate() {
        let state = if index == 0 {
            S::NoSave
        } else {
            S::Unavailable
        };
        for o in [80, 88, 96, 104, 112, 120] {
            let mut raw = golden;
            word64(&mut raw, o, 1);
            deny_chrome(&raw, E::State);
        }
        let mut raw = golden;
        word32(&mut raw, 240, 1);
        deny_chrome(&raw, E::State);
        // Every byte, not just the first word, in all three forbidden hashes.
        for o in 144..240 {
            let mut raw = golden;
            raw[o] = 1;
            deny_chrome(&raw, E::State);
        }
        for i in 0..10 {
            let mut c = chrome(state);
            match i {
                0 => c.original_backend_epoch = 1,
                1 => c.operation_id = 1,
                2 => c.source_handle = 1,
                3 => c.source_generation = 1,
                4 => c.expected_revision = 1,
                5 => c.result_revision = 1,
                6 => c.source_body_len = 1,
                7 => c.source_content_hash[31] = 1,
                8 => c.expected_content_hash[31] = 1,
                9 => c.receipt_sha256[31] = 1,
                _ => unreachable!(),
            }
            deny_chrome_typed(&c, E::State);
        }
    }
    for state in [S::Unknown, S::Committed, S::DefinitiveNoncommit] {
        let golden = chrome(state)
            .encode_le()
            .expect("positive synthetic control");
        for o in [80, 88, 112] {
            let mut raw = golden;
            word64(&mut raw, o, 0);
            deny_chrome(&raw, E::State);
        }
        for bad in [
            0,
            0x0100_0003_0000_0004,
            0x0201_0003_0000_0004,
            0x0200_0000_0000_0004,
            0x0200_0003_0000_0000,
        ] {
            let mut raw = golden;
            word64(&mut raw, 96, bad);
            deny_chrome(&raw, E::Handle);
        }
        for gen in [0, 5, 1u64 << 32, u64::MAX] {
            let mut raw = golden;
            word64(&mut raw, 104, gen);
            deny_chrome(&raw, E::Handle);
        }
        for len in [0, 4096] {
            let mut c = chrome(state);
            c.source_body_len = len;
            c.source_content_hash = [0; 32];
            c.expected_content_hash = [0; 32];
            assert_eq!(c.validate(), Ok(()));
        }
        let mut c = chrome(state);
        c.source_body_len = 4097;
        deny_chrome_typed(&c, E::Bounds);
    }
    let mut unknown = chrome(S::Unknown);
    unknown.source_handle = 0;
    unknown.source_generation = 0;
    unknown.source_body_len = 0;
    unknown.source_content_hash = [0; 32];
    // Known allocation metadata without submitted source is outside this codec.
    deny_chrome_typed(&unknown, E::Handle);
    unknown = chrome(S::Unknown);
    unknown.result_revision = 1;
    deny_chrome_typed(&unknown, E::State);
    for o in 208..240 {
        let mut raw = UNKNOWN_GOLDEN;
        raw[o] = 1;
        deny_chrome(&raw, E::State);
    }
    let mut raw = UNKNOWN_GOLDEN;
    word64(&mut raw, 80, 0);
    word32(&mut raw, 240, 4097);
    word64(&mut raw, 96, 0);
    deny_chrome(&raw, E::State);
    word64(&mut raw, 80, 8);
    deny_chrome(&raw, E::Bounds);
    word32(&mut raw, 240, 3);
    deny_chrome(&raw, E::Handle);
    // Opaque zero hashes are valid data even though no content or authority was proved.
    let mut c = chrome(S::Unknown);
    c.source_content_hash = [0; 32];
    c.expected_content_hash = [0; 32];
    assert_eq!(c.validate(), Ok(()));
}

#[test]
fn native_save_chrome_receipt_and_original_binding() {
    let b = binding();
    assert_eq!(b.validate(), Ok(()));
    for state in [S::Unknown, S::Committed, S::DefinitiveNoncommit] {
        let c = chrome(state);
        assert_ne!(c.current_store_epoch, c.original_backend_epoch);
        assert_eq!(c.validate_binding(&b, 8), Ok(()));
        assert_eq!(c.validate_binding(&b, 0), Err(E::Epoch));
        assert_eq!(c.validate_binding(&b, 9), Err(E::Binding));
        for i in 0..15 {
            // Every changed tuple is otherwise structurally valid: mismatch cannot
            // be explained by invalid zero IDs or malformed reference alone.
            let mut changed = b.clone();
            match i {
                0 => changed.allocation.actor.owner_id += 1,
                1 => changed.allocation.actor.session_id += 1,
                2 => changed.allocation.actor.session_generation += 1,
                3 => changed.allocation.actor.instance_id += 1,
                4 => changed.allocation.actor.instance_generation += 1,
                5 => changed.allocation.selected_object_id += 1,
                6 => changed.allocation.selected_generation += 1,
                7 => changed.allocation.operation_id += 1,
                8 => changed.allocation.expected_revision += 1,
                9 => changed.allocation.expected_content_hash[31] ^= 1,
                10 => changed.source.source_object_id = 0x0200_0005_0000_0004,
                11 => {
                    changed.source.source_object_id = 0x0200_0003_0000_0005;
                    changed.source.source_generation = 5;
                }
                12 => changed.source.byte_len += 1,
                13 => changed.source.content_hash[31] ^= 1,
                14 => {
                    changed.allocation.actor.owner_id = u64::MAX;
                    changed.allocation.selected_generation = u64::MAX;
                }
                _ => unreachable!(),
            }
            assert_eq!(changed.validate(), Ok(()));
            assert_eq!(c.validate_binding(&changed, 8), Err(E::Binding));
        }
        for i in 0..15 {
            let mut changed = c.clone();
            match i {
                0 => changed.actor.owner_id += 1,
                1 => changed.actor.session_id += 1,
                2 => changed.actor.session_generation += 1,
                3 => changed.actor.instance_id += 1,
                4 => changed.actor.instance_generation += 1,
                5 => changed.selected_object_id += 1,
                6 => changed.selected_object_generation += 1,
                7 => changed.operation_id += 1,
                8 => {
                    changed.expected_revision += 1;
                    if state == S::Committed {
                        changed.result_revision += 1;
                    }
                }
                9 => changed.expected_content_hash[31] ^= 1,
                10 => changed.source_handle = 0x0200_0005_0000_0004,
                11 => {
                    changed.source_handle = 0x0200_0003_0000_0005;
                    changed.source_generation = 5;
                }
                12 => changed.source_body_len += 1,
                13 => changed.source_content_hash[31] ^= 1,
                14 => changed.original_backend_epoch += 1,
                _ => unreachable!(),
            }
            assert_eq!(changed.validate(), Ok(()));
            assert_eq!(changed.validate_binding(&b, 8), Err(E::Binding));
        }
        // Binding validation priority precedes expected epoch and raw source check.
        let mut invalid = b.clone();
        invalid.schema_version = 2;
        assert_eq!(c.validate_binding(&invalid, 0), Err(E::Binding));
        invalid = b.clone();
        invalid.source.source_object_id = 0x0100_0003_0000_0004;
        assert_eq!(invalid.validate(), Ok(()));
        assert_eq!(c.validate_binding(&invalid, 8), Err(E::Handle));
        assert_eq!(c.validate_binding(&invalid, 0), Err(E::Epoch));
        for i in 0..4 {
            let mut invalid = b.clone();
            match i {
                0 => invalid.allocation.actor.owner_id = 0,
                1 => invalid.source.byte_len = 4097,
                2 => invalid.source.source_generation = 1u64 << 32,
                3 => invalid.allocation.schema_version = 2,
                _ => unreachable!(),
            }
            assert_eq!(c.validate_binding(&invalid, 8), Err(E::Binding));
        }
        for raw in [
            0x0200_0000_0000_0004,
            0x0201_0003_0000_0004,
            0x0200_0003_0000_0000,
        ] {
            let mut invalid = b.clone();
            invalid.source.source_object_id = raw;
            assert_eq!(invalid.validate(), Ok(()));
            assert_eq!(c.validate_binding(&invalid, 8), Err(E::Handle));
        }
        invalid = b.clone();
        invalid.source.source_generation = 5;
        assert_eq!(invalid.validate(), Ok(()));
        assert_eq!(c.validate_binding(&invalid, 8), Err(E::Handle));
    }
    for state in [S::NoSave, S::Unavailable] {
        assert_eq!(chrome(state).validate_binding(&b, 8), Err(E::State));
    }
    for state in [S::Committed, S::DefinitiveNoncommit] {
        let (c, bytes) = with_receipt(state);
        assert_eq!(c.validate_receipt(&b, 8, &bytes), Ok(()));
        assert_eq!(receipt(state).committed_at_ms, 0);
        // A reopened current epoch is independent of the original operation epoch.
        let mut reopened = c.clone();
        reopened.current_store_epoch = u64::MAX;
        assert_eq!(reopened.validate_receipt(&b, 8, &bytes), Ok(()));
        for len in 0..176 {
            assert_eq!(c.validate_receipt(&b, 8, &bytes[..len]), Err(E::Receipt));
        }
        let mut tail = bytes.to_vec();
        tail.push(0);
        assert_eq!(c.validate_receipt(&b, 8, &tail), Err(E::Receipt));
        for (o, v) in [(0, 0), (0, 2), (4, 175), (4, 177), (8, 2), (12, 1)] {
            let mut raw = bytes;
            word32(&mut raw, o, v);
            assert_eq!(c.validate_receipt(&b, 8, &raw), Err(E::Receipt));
        }
        for o in [16, 24, 32, 40, 48, 56, 64, 72, 80, 96] {
            let mut raw = bytes;
            word64(&mut raw, o, 0);
            assert_eq!(c.validate_receipt(&b, 8, &raw), Err(E::Receipt));
        }
        for o in [16, 24, 32, 40, 48, 56, 64, 72, 96] {
            let mut r = receipt(state);
            let mut raw = r.encode_le().unwrap();
            // Positive changed epoch/actor/object/op fields still decode.
            word64(&mut raw, o, u64::MAX);
            r = EditorSaveReceiptV0::decode_le(&raw).unwrap();
            assert_eq!(r.validate(), Ok(()));
            assert_eq!(c.validate_receipt(&b, 8, &raw), Err(E::Binding));
        }
        let mut invalid_outcome = bytes;
        if state == S::Committed {
            word64(&mut invalid_outcome, 88, 0);
        } else {
            word64(&mut invalid_outcome, 104, 1);
        }
        assert_eq!(c.validate_receipt(&b, 8, &invalid_outcome), Err(E::Receipt));
        for o in [112, 143, 144, 175] {
            let mut raw = bytes;
            raw[o] ^= 1;
            assert!(EditorSaveReceiptV0::decode_le(&raw).is_ok());
            assert_eq!(c.validate_receipt(&b, 8, &raw), Err(E::Binding));
        }
        let mut wrongbase = receipt(state);
        wrongbase.expected_revision = 20;
        wrongbase.result_revision = if state == S::Committed { 21 } else { 0 };
        let wrong = wrongbase.encode_le().unwrap();
        assert_eq!(c.validate_receipt(&b, 8, &wrong), Err(E::Binding));
        let other = if state == S::Committed {
            S::DefinitiveNoncommit
        } else {
            S::Committed
        };
        let opposite = receipt(other).encode_le().unwrap();
        assert_eq!(c.validate_receipt(&b, 8, &opposite), Err(E::Binding));
        let mut hash = c.clone();
        hash.receipt_sha256[31] ^= 1;
        assert_eq!(hash.validate(), Ok(()));
        assert_eq!(hash.validate_receipt(&b, 8, &bytes), Err(E::Binding));
        // Even recomputing SHA cannot bypass receipt tuple/outcome joins.
        let mut wrong = bytes;
        word64(&mut wrong, 72, 99);
        let mut matched_hash = c.clone();
        matched_hash.receipt_sha256 = digest(&wrong);
        assert_eq!(
            matched_hash.validate_receipt(&b, 8, &wrong),
            Err(E::Binding)
        );
        let mut malformed = bytes;
        word32(&mut malformed, 0, 2);
        let mut bad_binding = b.clone();
        bad_binding.allocation.operation_id += 1;
        assert_eq!(
            c.validate_receipt(&bad_binding, 8, &malformed),
            Err(E::Binding)
        );
        // Handle/generation/length are not in176: completeBinding must still join them.
        for i in 0..3 {
            let mut no_source = b.clone();
            match i {
                0 => no_source.source.byte_len = 4,
                1 => no_source.source.source_object_id = 0x0200_0005_0000_0004,
                2 => {
                    no_source.source.source_object_id = 0x0200_0003_0000_0005;
                    no_source.source.source_generation = 5;
                }
                _ => unreachable!(),
            }
            assert_eq!(receipt(state).validate_binding(&no_source), Ok(()));
            assert_eq!(c.validate_receipt(&no_source, 8, &bytes), Err(E::Binding));
        }
    }
    for state in [S::Unknown, S::NoSave, S::Unavailable] {
        assert_eq!(chrome(state).validate_receipt(&b, 8, &[]), Err(E::State));
    }
    let mut bad = chrome(S::Unknown);
    bad.reserved = 1;
    assert_eq!(bad.validate_binding(&b, 0), Err(E::Reserved));
    assert_eq!(bad.validate_receipt(&b, 0, &[]), Err(E::Reserved));
}

#[test]
fn native_save_chrome_successor_and_epoch_limits() {
    let b = binding();
    let mut c = chrome(S::Committed);
    c.expected_revision = u64::MAX - 1;
    c.result_revision = u64::MAX;
    assert_eq!(c.validate(), Ok(()));
    assert_eq!(Chrome::decode_le(&c.encode_le().unwrap()), Ok(c.clone()));
    let mut high = b.clone();
    high.allocation.expected_revision = u64::MAX - 1;
    let mut r = receipt(S::Committed);
    r.expected_revision = u64::MAX - 1;
    r.result_revision = u64::MAX;
    let raw = r.encode_le().unwrap();
    c.receipt_sha256 = digest(&raw);
    assert_eq!(c.validate_receipt(&high, 8, &raw), Ok(()));
    c.expected_revision = u64::MAX;
    c.result_revision = 0;
    deny_chrome_typed(&c, E::Overflow);
    let mut raw = COMMITTED_GOLDEN;
    word64(&mut raw, 112, u64::MAX);
    word64(&mut raw, 120, 0);
    deny_chrome(&raw, E::Overflow);
    for result in [0, 11, 13, u64::MAX] {
        let mut c = chrome(S::Committed);
        c.result_revision = result;
        deny_chrome_typed(&c, E::Overflow);
    }
    for state in [S::Unknown, S::DefinitiveNoncommit] {
        let mut c = chrome(state);
        c.expected_revision = u64::MAX;
        assert_eq!(c.validate(), Ok(()));
        for result in [1, u64::MAX] {
            let mut changed = c.clone();
            changed.result_revision = result;
            deny_chrome_typed(&changed, E::State);
        }
    }
    for state in [S::Unknown, S::Committed, S::DefinitiveNoncommit] {
        for (handle, gen) in [
            (0x0200_0001_0000_0001, 1),
            (0x0200_ffff_ffff_ffff, u32::MAX as u64),
        ] {
            let mut c = chrome(state);
            c.source_handle = handle;
            c.source_generation = gen;
            assert_eq!(c.validate(), Ok(()));
        }
        let mut c = chrome(state);
        c.source_body_len = 4097;
        c.source_handle = 0;
        deny_chrome_typed(&c, E::Bounds);
        c = chrome(state);
        c.source_handle = 0;
        c.result_revision = if state == S::Committed { 0 } else { 1 };
        deny_chrome_typed(
            &c,
            if state == S::Committed {
                E::Handle
            } else {
                E::State
            },
        );
    }
    for state in [S::Committed, S::DefinitiveNoncommit] {
        let mut c = chrome(state);
        c.receipt_sha256 = [0; 32];
        assert_eq!(c.validate(), Ok(()));
        let raw = receipt(state).encode_le().unwrap();
        assert_eq!(c.validate_receipt(&b, 8, &raw), Err(E::Binding));
        let (c, raw) = with_receipt(state);
        assert_eq!(c.validate_receipt(&b, 8, &raw), Ok(()));
    }
    // Unknown has no transition API or receipt; even a known noncommit receipt
    // does not reclassify this unchanged Unknown record. A distinct supplied
    // definitive record must perform the exact original tuple+receipt join.
    let unknown = chrome(S::Unknown);
    let bytes = receipt(S::DefinitiveNoncommit).encode_le().unwrap();
    assert_eq!(unknown.validate_receipt(&b, 8, &bytes), Err(E::State));
    assert_eq!(unknown.state, S::Unknown);
    assert_eq!(unknown.receipt_sha256, [0; 32]);
    let mut alleged = unknown.clone();
    alleged.state = S::DefinitiveNoncommit;
    assert_eq!(alleged.validate(), Ok(()));
    assert_eq!(alleged.validate_receipt(&b, 8, &[]), Err(E::Receipt));
    assert_eq!(alleged.validate_receipt(&b, 8, &bytes), Err(E::Binding));
    alleged.receipt_sha256 = digest(&bytes);
    assert_eq!(alleged.validate_receipt(&b, 8, &bytes), Ok(()));
    // This last success is pure correlation of supplied data, not a journal or
    // authority proof that the original timed-out execution did not commit.
}

#[test]
fn native_save_read_codecs_remain_distinct() {
    for save in [PREVIEW_GOLDEN, ACTIVE_GOLDEN] {
        let mut old = save;
        word32(&mut old, 0, 2);
        word32(&mut old, 452, 1);
        let d = read::EditorPreviewDataV0::decode_le(&old).expect("unchanged schema2 Read data");
        assert_eq!(d.records[3].class, read::EditorPreviewClassV0::StoreRead);
        assert_eq!(d.records[3].rights, 1);
        assert_eq!(d.encode_le(), Ok(old));
        assert_eq!(
            read::EditorPreviewDataV0::decode_le(&save),
            Err(read::EditorPreviewErrorV0::Version)
        );
        assert_eq!(Data::decode_le(&old), Err(E::Version));
        let mut wrong_rights = old;
        word32(&mut wrong_rights, 452, 7);
        assert_eq!(
            read::EditorPreviewDataV0::decode_le(&wrong_rights),
            Err(read::EditorPreviewErrorV0::Constants)
        );
        let mut only_version = old;
        word32(&mut only_version, 0, 3);
        assert_eq!(Data::decode_le(&only_version), Err(E::Constants));
    }
    for (index, save) in [NO_SAVE_GOLDEN, UNAVAILABLE_GOLDEN].into_iter().enumerate() {
        let state = if index == 0 {
            S::NoSave
        } else {
            S::Unavailable
        };
        let mut old = save;
        word32(&mut old, 0, 1);
        let c =
            read::EditorPreviewChromeV0::decode_le(&old).expect("unchanged schema1 current status");
        assert_eq!(c.encode_le(), Ok(old));
        assert_eq!(Chrome::decode_le(&old), Err(E::Version));
        assert_eq!(
            read::EditorPreviewChromeV0::decode_le(&save),
            Err(read::EditorPreviewErrorV0::Version)
        );
        assert_eq!(c.state as u32, state as u32);
        for operation in [UNKNOWN_GOLDEN, COMMITTED_GOLDEN, NONCOMMIT_GOLDEN] {
            let mut wrong = operation;
            word32(&mut wrong, 0, 1);
            assert_eq!(
                read::EditorPreviewChromeV0::decode_le(&wrong),
                Err(read::EditorPreviewErrorV0::State)
            );
        }
    }
    // Existing shared text64 and receipts176 keep their versions and semantics.
    let text = EditorTextHeaderV0::try_new(6, 11, b"").unwrap();
    let mut text_golden = [0; 64];
    word32(&mut text_golden, 0, 1);
    word32(&mut text_golden, 4, 64);
    word64(&mut text_golden, 8, 6);
    word64(&mut text_golden, 16, 11);
    text_golden[24..56].copy_from_slice(&EMPTY_HASH);
    assert_eq!(text.encode_le(), Ok(text_golden));
    assert_eq!(EditorTextHeaderV0::decode_le(&text_golden), Ok(text));
    for state in [S::Committed, S::DefinitiveNoncommit] {
        let r = receipt(state);
        let wire = r.encode_le().unwrap();
        assert_eq!(wire.len(), 176);
        assert_eq!(&wire[..8], &[1, 0, 0, 0, 176, 0, 0, 0]);
        assert_eq!(EditorSaveReceiptV0::decode_le(&wire), Ok(r));
    }
    // Static pin/control-contract checks only, not an executed old-client claim.
    assert_eq!(
        include_str!("../../idl/portals/desktop_editor_session_v1.toml")
            .lines()
            .find(|s| s.starts_with("protocol =")),
        Some("protocol = 352 # 0x160; incompatible versions require a new protocol ID.")
    );
    let artifact = include_str!("../../idl/portals/desktop_artifact_v1.toml");
    assert!(
        artifact
            .contains("protocol = 368 # 0x170; incompatible versions require a new protocol ID.")
    );
    for (name, id) in [
        ("read_selected", 1),
        ("read_selected_reply", 2),
        ("allocate_save_id", 3),
        ("allocate_save_id_reply", 4),
        ("commit", 5),
        ("commit_reply", 6),
        ("save_status", 7),
        ("save_status_reply", 8),
    ] {
        assert!(artifact.contains(&format!("[message.{name}]\nmsg_type = {id}\n")));
    }
    // A coherent counterfeit tuple can pass data consistency. There is no host
    // registry, issuer, current clock, permit, IO or publisher in these tests.
    let (c, wire) = with_receipt(S::Committed);
    assert_eq!(c.validate_receipt(&binding(), 8, &wire), Ok(()));
}
