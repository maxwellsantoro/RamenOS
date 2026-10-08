//! Pure shared-data contract assertions. No Core/Gate/clock/IO authority is exercised.
//! Golden bytes below are literal LE fixtures independent of the codec under test.
use artifact_store_schema::editor_preview::{
    APP_BOTTOM, APP_TOP, CHROME_SCHEMA_VERSION, CHROME_STATUS_LEN, COMPOSED_HEIGHT, COMPOSED_WIDTH,
    DATA_PROTOCOL_VERSION, EditorPreviewChromeStateV0 as S, EditorPreviewChromeV0 as Chrome,
    EditorPreviewClassV0 as C, EditorPreviewDataV0 as Data, EditorPreviewErrorV0 as E,
    EditorPreviewGrantV0 as Grant, EditorPreviewPhaseV0 as P, INPUT_CAPACITY, MAX_TEXT_LEN,
    PREVIEW_CATEGORY_MASK, PREVIEW_DATA_LEN, PREVIEW_RECORD_COUNT, PREVIEW_SCHEMA_VERSION,
    SURFACE_FORMAT, SURFACE_HEIGHT, SURFACE_STRIDE, SURFACE_WIDTH, decode_ipc_reference,
    encode_ipc_reference,
};
use artifact_store_schema::editor_save::EditorActorV0;

const PREVIEW_GOLDEN: [u8; 464] = [
    0x02, 0x00, 0x00, 0x00, 0xd0, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
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
    0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 448..464
];

const ACTIVE_GOLDEN: [u8; 464] = [
    0x02, 0x00, 0x00, 0x00, 0xd0, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
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
    0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 448..464
];

const NO_SAVE_GOLDEN: [u8; 248] = [
    0x01, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 0..16
    0xa8, 0xa7, 0xa6, 0xa5, 0xa4, 0xa3, 0xa2, 0xa1, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02,
    0x01, // 16..32
    0x10, 0x0f, 0x0e, 0x0d, 0x0c, 0x0b, 0x0a, 0x09, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12,
    0x11, // 32..48
    0x20, 0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a, 0x19, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22,
    0x21, // 48..64
    0x30, 0x2f, 0x2e, 0x2d, 0x2c, 0x2b, 0x2a, 0x29, 0x68, 0x67, 0x66, 0x65, 0x64, 0x63, 0x62,
    0x61, // 64..80
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 80..96
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 96..112
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 112..128
    0xb8, 0xb7, 0xb6, 0xb5, 0xb4, 0xb3, 0xb2, 0xb1, 0xc8, 0xc7, 0xc6, 0xc5, 0xc4, 0xc3, 0xc2,
    0xc1, // 128..144
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

const UNAVAILABLE_GOLDEN: [u8; 248] = [
    0x01, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 0..16
    0xa8, 0xa7, 0xa6, 0xa5, 0xa4, 0xa3, 0xa2, 0xa1, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02,
    0x01, // 16..32
    0x10, 0x0f, 0x0e, 0x0d, 0x0c, 0x0b, 0x0a, 0x09, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12,
    0x11, // 32..48
    0x20, 0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a, 0x19, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22,
    0x21, // 48..64
    0x30, 0x2f, 0x2e, 0x2d, 0x2c, 0x2b, 0x2a, 0x29, 0x68, 0x67, 0x66, 0x65, 0x64, 0x63, 0x62,
    0x61, // 64..80
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 80..96
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 96..112
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, // 112..128
    0xb8, 0xb7, 0xb6, 0xb5, 0xb4, 0xb3, 0xb2, 0xb1, 0xc8, 0xc7, 0xc6, 0xc5, 0xc4, 0xc3, 0xc2,
    0xc1, // 128..144
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
    let classes = [C::SelfStatus, C::FocusRead, C::Surface, C::StoreRead];
    let rights = [1, 1, 15, 1];
    Data {
        phase,
        schema_version: 2,
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
fn chrome_fixture(state: S) -> Chrome {
    Chrome {
        schema_version: 1,
        total_len: 248,
        state,
        reserved: 0,
        actor: EditorActorV0 {
            owner_id: 0xa1a2_a3a4_a5a6_a7a8,
            session_id: SESSION,
            session_generation: SESSION_GEN,
            instance_id: INSTANCE,
            instance_generation: INSTANCE_GEN,
        },
        selected_object_id: OBJECT,
        selected_object_generation: OBJECT_GEN,
        current_store_epoch: STORE_EPOCH,
        original_backend_epoch: 0,
        operation_id: 0,
        source_handle: 0,
        source_generation: 0,
        expected_revision: 0,
        result_revision: 0,
        attachment_version: 0xb1b2_b3b4_b5b6_b7b8,
        status_version: 0xc1c2_c3c4_c5c6_c7c8,
        source_content_hash: [0; 32],
        expected_content_hash: [0; 32],
        receipt_sha256: [0; 32],
        source_body_len: 0,
        tail_reserved: 0,
    }
}
fn word32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn word64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn deny_data(bytes: &[u8], expected: E) {
    assert_eq!(Data::decode_le(bytes), Err(expected));
    assert_eq!(Data::decode_le_for_phase(bytes, P::Preview), Err(expected));
    assert_eq!(Data::decode_le_for_phase(bytes, P::Active), Err(expected));
}
fn deny_typed(data: &Data, expected: E) {
    assert_eq!(data.validate(), Err(expected));
    assert_eq!(data.encode_le(), Err(expected));
    assert_eq!(data.validate_for_phase(P::Preview), Err(expected));
    assert_eq!(data.validate_for_phase(P::Active), Err(expected));
}
fn positive() {
    for (phase, golden) in [(P::Preview, &PREVIEW_GOLDEN), (P::Active, &ACTIVE_GOLDEN)] {
        let data = fixture(phase);
        assert_eq!(data.validate(), Ok(()));
        assert_eq!(data.validate_for_phase(phase), Ok(()));
        assert_eq!(data.encode_le(), Ok(*golden));
        assert_eq!(Data::decode_le(golden), Ok(data));
        assert_eq!(Data::decode_le_for_phase(golden, phase), Ok(data));
    }
}
fn deny_chrome(bytes: &[u8], expected: E) {
    assert_eq!(Chrome::decode_le(bytes), Err(expected));
}
fn deny_chrome_typed(data: &Chrome, expected: E) {
    assert_eq!(data.validate(), Err(expected));
    assert_eq!(data.encode_le(), Err(expected));
}

#[test]
fn preview_schema2_golden_layout_and_phases() {
    assert_eq!(
        (
            PREVIEW_SCHEMA_VERSION,
            PREVIEW_DATA_LEN,
            PREVIEW_RECORD_COUNT,
            PREVIEW_CATEGORY_MASK
        ),
        (2, 464, 4, 63)
    );
    assert_eq!((CHROME_SCHEMA_VERSION, CHROME_STATUS_LEN), (1, 248));
    assert_eq!(
        (MAX_TEXT_LEN, SURFACE_WIDTH, SURFACE_HEIGHT, SURFACE_STRIDE),
        (4096, 640, 480, 2560)
    );
    assert_eq!(
        (SURFACE_FORMAT, INPUT_CAPACITY, DATA_PROTOCOL_VERSION),
        (1, 64, 1)
    );
    assert_eq!(
        (COMPOSED_WIDTH, COMPOSED_HEIGHT, APP_TOP, APP_BOTTOM),
        (640, 568, 48, 528)
    );
    positive();
    assert_eq!(
        Data::decode_le_for_phase(&PREVIEW_GOLDEN, P::Active),
        Err(E::Phase)
    );
    assert_eq!(
        Data::decode_le_for_phase(&ACTIVE_GOLDEN, P::Preview),
        Err(E::Phase)
    );
    assert_eq!(
        fixture(P::Preview).validate_for_phase(P::Active),
        Err(E::Phase)
    );
    assert_eq!(
        fixture(P::Active).validate_for_phase(P::Preview),
        Err(E::Phase)
    );
    let mut data = fixture(P::Preview);
    data.application_hash = [0; 32];
    data.manifest_hash = [0; 32];
    data.selected_content_hash = [0; 32];
    let mut literal = PREVIEW_GOLDEN;
    literal[112..208].fill(0);
    assert_eq!(data.encode_le(), Ok(literal));
    assert_eq!(Data::decode_le(&literal), Ok(data));
    assert_eq!(fixture(P::Preview).selected_content_hash, EMPTY_HASH);
}

#[test]
fn preview_schema2_lengths_version_constants_reserved() {
    for golden in [&PREVIEW_GOLDEN, &ACTIVE_GOLDEN] {
        for length in 0..464 {
            deny_data(&golden[..length], E::Length);
        }
        let mut tail = golden.to_vec();
        tail.push(0);
        deny_data(&tail, E::Length);
        for length in [0, 463, 465, u32::MAX] {
            let mut raw = *golden;
            word32(&mut raw, 4, length);
            deny_data(&raw, E::Length);
        }
        for version in [0, 1, 3, u32::MAX] {
            let mut raw = *golden;
            word32(&mut raw, 0, version);
            deny_data(&raw, E::Version);
        }
        for (offset, value) in [
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
            let mut raw = *golden;
            word32(&mut raw, offset, value);
            deny_data(&raw, E::Constants);
        }
        for offset in [236, 292, 348, 404, 460] {
            let mut raw = *golden;
            word32(&mut raw, offset, 1);
            deny_data(&raw, E::Reserved);
        }
        for i in 0..4 {
            for (relative, value) in [(40, 0), (40, 5), (44, 0), (44, 7), (48, 0), (48, 2)] {
                let mut raw = *golden;
                word32(&mut raw, 240 + i * 56 + relative, value);
                deny_data(&raw, E::Constants);
            }
        }
    }
    let base = fixture(P::Active);
    for field in 0..14 {
        let mut data = base;
        match field {
            0 => data.total_len = 463,
            1 => data.schema_version = 1,
            2 => data.reserved = 1,
            3 => data.category_mask = 62,
            4 => data.record_count = 5,
            5 => data.max_text_len = 4095,
            6 => data.surface_width = 639,
            7 => data.surface_height = 479,
            8 => data.surface_stride = 2559,
            9 => data.surface_format = 0,
            10 => data.input_capacity = 65,
            11 => data.protocol_version = 2,
            12 => data.records[3].rights = 7,
            13 => data.records[2].reserved = 1,
            _ => unreachable!(),
        }
        deny_typed(
            &data,
            match field {
                0 => E::Length,
                1 => E::Version,
                2 | 13 => E::Reserved,
                _ => E::Constants,
            },
        );
    }
    // Declared global precedence, not merely independent single-property errors.
    let mut raw = ACTIVE_GOLDEN;
    word32(&mut raw, 4, 463);
    word32(&mut raw, 0, 1);
    word32(&mut raw, 236, 1);
    deny_data(&raw, E::Length);
    word32(&mut raw, 4, 464);
    deny_data(&raw, E::Version);
    word32(&mut raw, 0, 2);
    deny_data(&raw, E::Reserved);
    word32(&mut raw, 236, 0);
    word32(&mut raw, 8, 62);
    word64(&mut raw, 40, 0);
    deny_data(&raw, E::Constants);
    positive();
}

#[test]
fn preview_schema2_phase_and_identity_denials() {
    for golden in [&PREVIEW_GOLDEN, &ACTIVE_GOLDEN] {
        for (instance, generation) in [(0, 1), (1, 0)] {
            let mut raw = *golden;
            word64(&mut raw, 32, instance);
            word64(&mut raw, 40, generation);
            deny_data(&raw, E::Phase);
        }
        for offset in [16, 24, 48, 56, 64, 72, 80, 88, 96, 104] {
            let mut raw = *golden;
            word64(&mut raw, offset, 0);
            deny_data(&raw, E::Identity);
        }
        for i in 0..4 {
            let mut raw = *golden;
            word32(&mut raw, 280 + i * 56, if i == 0 { 2 } else { 1 });
            deny_data(&raw, E::Constants);
        }
    }
    for phase in [P::Preview, P::Active] {
        let mut data = fixture(phase);
        data.phase = if phase == P::Preview {
            P::Active
        } else {
            P::Preview
        };
        deny_typed(&data, E::Phase);
    }
    for i in 0..4 {
        let mut preview = PREVIEW_GOLDEN;
        word64(&mut preview, 240 + i * 56, 0x0100_0000_0000_0001);
        deny_data(&preview, E::Handle);
        let mut active = ACTIVE_GOLDEN;
        word64(&mut active, 240 + i * 56, 0);
        deny_data(&active, E::Handle);
    }
    let mut raw = ACTIVE_GOLDEN;
    word64(&mut raw, 40, 0);
    word64(&mut raw, 16, 0);
    deny_data(&raw, E::Phase);
    word64(&mut raw, 40, INSTANCE_GEN);
    word64(&mut raw, 240, 0);
    deny_data(&raw, E::Identity);
    positive();
}

#[test]
fn preview_schema2_resource_epoch_expiry_bindings() {
    for phase in [P::Preview, P::Active] {
        let golden = if phase == P::Preview {
            PREVIEW_GOLDEN
        } else {
            ACTIVE_GOLDEN
        };
        for i in 0..2 {
            for relative in [8, 16] {
                let mut raw = golden;
                word64(&mut raw, 240 + i * 56 + relative, 1);
                deny_data(&raw, E::Resource);
            }
        }
        for relative in [8, 16] {
            let mut raw = golden;
            word64(
                &mut raw,
                352 + relative,
                if phase == P::Preview { 1 } else { 0 },
            );
            deny_data(&raw, E::Resource);
            let mut raw = golden;
            word64(&mut raw, 408 + relative, 1);
            deny_data(&raw, E::Resource);
        }
        for i in 0..3 {
            for epoch in [0, DESKTOP_EPOCH + 1] {
                let mut raw = golden;
                word64(&mut raw, 264 + i * 56, epoch);
                deny_data(&raw, E::Epoch);
            }
        }
        let mut raw = golden;
        word64(&mut raw, 432, 0);
        deny_data(&raw, E::Epoch);
        for i in 0..4 {
            for expires in [0, EXPIRES - 1] {
                let mut raw = golden;
                word64(&mut raw, 272 + i * 56, expires);
                deny_data(&raw, E::Expiry);
            }
        }
        let mut data = fixture(phase);
        data.records[3].service_epoch = DESKTOP_EPOCH;
        let mut same_epoch = golden;
        word64(&mut same_epoch, 432, DESKTOP_EPOCH);
        assert_eq!(data.encode_le(), Ok(same_epoch));
        assert_eq!(Data::decode_le(&same_epoch), Ok(data));
        data = fixture(phase);
        data.records[3].resource_generation = 1;
        deny_typed(&data, E::Resource);
        data = fixture(phase);
        data.records[0].service_epoch = 0;
        deny_typed(&data, E::Epoch);
        data = fixture(phase);
        data.records[1].expires_at_ms = 0;
        deny_typed(&data, E::Expiry);
    }
    let mut wide = fixture(P::Active);
    wide.session_generation = u64::MAX;
    wide.instance_generation = u64::MAX;
    wide.selected_object_generation = u64::MAX;
    wide.records[0].resource_generation = u64::MAX;
    wide.records[1].resource_generation = u64::MAX;
    wide.records[2].resource_generation = u64::MAX;
    wide.records[3].resource_generation = u64::MAX;
    let mut raw = ACTIVE_GOLDEN;
    for offset in [24, 40, 56, 256, 312, 368, 424] {
        word64(&mut raw, offset, u64::MAX);
    }
    assert_eq!(wide.validate(), Ok(()));
    assert_eq!(wide.encode_le(), Ok(raw));
    assert_eq!(Data::decode_le(&raw), Ok(wide));
    // Handle before Resource, Resource before Epoch, Epoch before Expiry.
    let mut raw = ACTIVE_GOLDEN;
    word64(&mut raw, 240, 0);
    word64(&mut raw, 248, 0);
    word64(&mut raw, 264, 0);
    word64(&mut raw, 272, 0);
    deny_data(&raw, E::Handle);
    word64(&mut raw, 240, 0x0100_0000_0000_0001);
    deny_data(&raw, E::Resource);
    word64(&mut raw, 248, INSTANCE);
    deny_data(&raw, E::Epoch);
    word64(&mut raw, 264, DESKTOP_EPOCH);
    deny_data(&raw, E::Expiry);
    positive();
}

#[test]
fn preview_schema2_checked_handle_parts() {
    for (slot, generation, raw) in [
        (0, 1, 0x0100_0000_0000_0001),
        (65535, 1, 0x0100_ffff_0000_0001),
        (0, 0xffff_ffff, 0x0100_0000_ffff_ffff),
        (65535, 0xffff_ffff, 0x0100_ffff_ffff_ffff),
    ] {
        assert_eq!(encode_ipc_reference(slot, generation), Ok(raw));
        assert_eq!(decode_ipc_reference(raw), Ok((slot, generation)));
    }
    for slot in [65536, u32::MAX] {
        assert_eq!(encode_ipc_reference(slot, 1), Err(E::Handle));
    }
    for generation in [0, 0x1_0000_0000, u64::MAX] {
        assert_eq!(encode_ipc_reference(0, generation), Err(E::Handle));
    }
    let bad = [
        0,
        0x0100_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x0200_0000_0000_0001,
        0x0300_0000_0000_0001,
        0xff00_0000_0000_0001,
        0x0101_0000_0000_0001,
        0x01ff_0000_0000_0001,
    ];
    for raw in bad {
        assert_eq!(decode_ipc_reference(raw), Err(E::Handle));
        for i in 0..4 {
            let mut bytes = ACTIVE_GOLDEN;
            word64(&mut bytes, 240 + i * 56, raw);
            deny_data(&bytes, E::Handle);
        }
        let mut data = fixture(P::Active);
        data.records[3].handle = raw;
        deny_typed(&data, E::Handle);
    }
    // Width rejection establishes no repair of an already truncated foreign representation.
    assert_eq!(encode_ipc_reference(1, 1), Ok(0x0100_0001_0000_0001));
    positive();
}

#[test]
fn chrome_current_golden_layout_and_denials() {
    for (state, golden) in [
        (S::NoSave, &NO_SAVE_GOLDEN),
        (S::Unavailable, &UNAVAILABLE_GOLDEN),
    ] {
        let data = chrome_fixture(state);
        assert_eq!(data.validate(), Ok(()));
        assert_eq!(data.encode_le(), Ok(*golden));
        assert_eq!(Chrome::decode_le(golden), Ok(data.clone()));
        for length in 0..248 {
            deny_chrome(&golden[..length], E::Length);
        }
        let mut tail = golden.to_vec();
        tail.push(0);
        deny_chrome(&tail, E::Length);
        for length in [0, 247, 249, u32::MAX] {
            let mut raw = *golden;
            word32(&mut raw, 4, length);
            deny_chrome(&raw, E::Length);
        }
        for version in [0, 2, u32::MAX] {
            let mut raw = *golden;
            word32(&mut raw, 0, version);
            deny_chrome(&raw, E::Version);
        }
        for offset in [12, 244] {
            let mut raw = *golden;
            word32(&mut raw, offset, 1);
            deny_chrome(&raw, E::Reserved);
        }
        for value in [1, 2, 3, 5, u32::MAX] {
            let mut raw = *golden;
            word32(&mut raw, 8, value);
            deny_chrome(&raw, E::State);
        }
        for offset in [16, 24, 32, 40, 48, 56, 64, 72, 128, 136] {
            let mut raw = *golden;
            word64(&mut raw, offset, 0);
            deny_chrome(&raw, E::Identity);
        }
        for offset in [80, 88, 96, 104, 112, 120] {
            let mut raw = *golden;
            word64(&mut raw, offset, 1);
            deny_chrome(&raw, E::State);
        }
        for offset in 144..240 {
            let mut raw = *golden;
            raw[offset] = 1;
            deny_chrome(&raw, E::State);
        }
        let mut raw = *golden;
        word32(&mut raw, 240, 1);
        deny_chrome(&raw, E::State);
        for field in 0..14 {
            let mut changed = data.clone();
            match field {
                0 => changed.total_len = 247,
                1 => changed.schema_version = 2,
                2 => changed.reserved = 1,
                3 => changed.tail_reserved = 1,
                4 => changed.actor.owner_id = 0,
                5 => changed.original_backend_epoch = 1,
                6 => changed.operation_id = 1,
                7 => changed.source_handle = 1,
                8 => changed.source_generation = 1,
                9 => changed.expected_revision = 1,
                10 => changed.result_revision = 1,
                11 => changed.source_content_hash[0] = 1,
                12 => changed.receipt_sha256[31] = 1,
                13 => changed.source_body_len = 1,
                _ => unreachable!(),
            }
            deny_chrome_typed(
                &changed,
                match field {
                    0 => E::Length,
                    1 => E::Version,
                    2 | 3 => E::Reserved,
                    4 => E::Identity,
                    _ => E::State,
                },
            );
        }
        assert_eq!(data.encode_le(), Ok(*golden));
    }
    let mut raw = NO_SAVE_GOLDEN;
    word32(&mut raw, 4, 247);
    word32(&mut raw, 0, 2);
    word32(&mut raw, 12, 1);
    word32(&mut raw, 8, 2);
    word64(&mut raw, 16, 0);
    word64(&mut raw, 88, 1);
    deny_chrome(&raw, E::Length);
    word32(&mut raw, 4, 248);
    deny_chrome(&raw, E::Version);
    word32(&mut raw, 0, 1);
    deny_chrome(&raw, E::Reserved);
    word32(&mut raw, 12, 0);
    deny_chrome(&raw, E::State);
    word32(&mut raw, 8, 0);
    deny_chrome(&raw, E::Identity);
    word64(&mut raw, 16, 0xa1a2_a3a4_a5a6_a7a8);
    deny_chrome(&raw, E::State);
}

#[test]
fn shared_codec_pure_binding_and_version_boundary() {
    // Version boundary only. This does not run or mimic the old EditorClient.
    let mut old = PREVIEW_GOLDEN;
    word32(&mut old, 0, 1);
    deny_data(&old, E::Version);
    for future in [1, 2, 3] {
        let mut raw = NO_SAVE_GOLDEN;
        word32(&mut raw, 8, future);
        deny_chrome(&raw, E::State);
    }
    // Coherent arbitrary observations remain data: no registry, clock or issuer exists here.
    let mut observed = fixture(P::Active);
    observed.session_id = 99;
    observed.session_generation = u64::MAX;
    observed.instance_id = 101;
    observed.instance_generation = u64::MAX;
    observed.selected_object_id = 103;
    observed.selected_object_generation = u64::MAX;
    observed.expires_at_ms = 1;
    observed.application_hash = [0; 32];
    observed.manifest_hash = [0; 32];
    observed.selected_content_hash = [0xff; 32];
    for i in 0..2 {
        observed.records[i].resource_id = 101;
        observed.records[i].resource_generation = u64::MAX;
    }
    observed.records[3].resource_id = 103;
    observed.records[3].resource_generation = u64::MAX;
    observed.records[3].handle = observed.records[0].handle; // Separate host registries may share raw values.
    for record in &mut observed.records {
        record.expires_at_ms = 1;
    }
    let mut literal = ACTIVE_GOLDEN;
    for (offset, value) in [
        (16, 99),
        (24, u64::MAX),
        (32, 101),
        (40, u64::MAX),
        (48, 103),
        (56, u64::MAX),
        (96, 1),
        (248, 101),
        (256, u64::MAX),
        (304, 101),
        (312, u64::MAX),
        (408, 0x0100_0000_0000_0001),
        (416, 103),
        (424, u64::MAX),
    ] {
        word64(&mut literal, offset, value);
    }
    for offset in [272, 328, 384, 440] {
        word64(&mut literal, offset, 1);
    }
    literal[112..176].fill(0);
    literal[176..208].fill(0xff);
    assert_eq!(observed.validate(), Ok(()));
    assert_eq!(observed.encode_le(), Ok(literal));
    assert_eq!(Data::decode_le(&literal), Ok(observed));
    let mut current = chrome_fixture(S::NoSave);
    current.actor.owner_id = 99;
    current.actor.instance_generation = u64::MAX;
    current.current_store_epoch = 1;
    current.attachment_version = u64::MAX;
    current.status_version = u64::MAX;
    let mut bytes = NO_SAVE_GOLDEN;
    for (offset, value) in [
        (16, 99),
        (48, u64::MAX),
        (72, 1),
        (128, u64::MAX),
        (136, u64::MAX),
    ] {
        word64(&mut bytes, offset, value);
    }
    assert_eq!(current.validate(), Ok(()));
    assert_eq!(current.encode_le(), Ok(bytes));
    assert_eq!(Chrome::decode_le(&bytes), Ok(current));
    positive();
    // Actual Core/Gate/currentepoch/clock/lifecycle/owner denials belong to separate native-service tests.
}
