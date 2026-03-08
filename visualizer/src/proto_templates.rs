pub struct ProtoTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: &'static str,
    pub hex_data: &'static str,
}

pub fn all() -> &'static [ProtoTemplate] {
    &[
        PERSON,
        ADDRESS_BOOK,
        SEARCH_RESPONSE,
        ORDER,
        SENSOR_READING,
        NESTED_CONFIG,
        MIXED_WIRE_TYPES,
        PACKED_REPEATED,
    ]
}

// ---------------------------------------------------------------------------
// Template 1: Person (introductory)
// name="Alice", id=123, email="alice@example.com",
// phone { number="555-1234", type=MOBILE }
// ---------------------------------------------------------------------------

const PERSON: ProtoTemplate = ProtoTemplate {
    name: "Person",
    description: "Person with name, id, email, and a phone number",
    schema: r#"syntax = "proto3";

message Person {
  string name = 1;
  int32 id = 2;
  string email = 3;

  enum PhoneType {
    MOBILE = 0;
    HOME = 1;
    WORK = 2;
  }

  message PhoneNumber {
    string number = 1;
    PhoneType type = 2;
  }

  repeated PhoneNumber phones = 4;
}"#,
    // field 1 string "Alice":             0a 05 41 6c 69 63 65
    // field 2 varint 123:                 10 7b
    // field 3 string "alice@example.com": 1a 11 61 6c 69 63 65 40 65 78 61 6d 70 6c 65 2e 63 6f 6d
    // field 4 message (phone):            22 0c
    //   field 1 string "555-1234":          0a 08 35 35 35 2d 31 32 33 34
    //   field 2 varint 0 (MOBILE):          10 00
    hex_data: "0a 05 41 6c 69 63 65 10 7b 1a 11 61 6c 69 63 65 40 65 78 61 6d 70 6c 65 2e 63 6f 6d 22 0c 0a 08 35 35 35 2d 31 32 33 34 10 00",
};

// ---------------------------------------------------------------------------
// Template 2: Address Book
// Two people with multiple phone numbers
// ---------------------------------------------------------------------------

const ADDRESS_BOOK: ProtoTemplate = ProtoTemplate {
    name: "Address Book",
    description: "Two contacts with multiple phone numbers each",
    schema: r#"syntax = "proto3";

message PhoneNumber {
  string number = 1;
  int32 type = 2;
}

message Person {
  string name = 1;
  int32 id = 2;
  string email = 3;
  repeated PhoneNumber phones = 4;
}

message AddressBook {
  repeated Person people = 1;
}"#,
    // person 1: name="Bob", id=1, email="bob@test.com",
    //           phones=[{number="555-0001", type=1}]
    // person 2: name="Eve", id=2, email="eve@test.com",
    //           phones=[{number="555-0002", type=0}, {number="555-0003", type=2}]
    //
    // AddressBook { people: [person1, person2] }
    // field 1 (Person): 0a 23 (35 bytes)
    //   name="Bob":       0a 03 42 6f 62
    //   id=1:             10 01
    //   email="bob@test.com": 1a 0c 62 6f 62 40 74 65 73 74 2e 63 6f 6d
    //   phone {number="555-0001", type=1}: 22 0c 0a 08 35 35 35 2d 30 30 30 31 10 01
    // field 1 (Person): 0a 31 (49 bytes)
    //   name="Eve":       0a 03 45 76 65
    //   id=2:             10 02
    //   email="eve@test.com": 1a 0c 65 76 65 40 74 65 73 74 2e 63 6f 6d
    //   phone1 {number="555-0002", type=0}: 22 0c 0a 08 35 35 35 2d 30 30 30 32 10 00
    //   phone2 {number="555-0003", type=2}: 22 0c 0a 08 35 35 35 2d 30 30 30 33 10 02
    hex_data: "0a 23 0a 03 42 6f 62 10 01 1a 0c 62 6f 62 40 74 65 73 74 2e 63 6f 6d 22 0c 0a 08 35 35 35 2d 30 30 30 31 10 01 0a 31 0a 03 45 76 65 10 02 1a 0c 65 76 65 40 74 65 73 74 2e 63 6f 6d 22 0c 0a 08 35 35 35 2d 30 30 30 32 10 00 22 0c 0a 08 35 35 35 2d 30 30 30 33 10 02",
};

// ---------------------------------------------------------------------------
// Template 3: Search Response
// Multiple results with snippets and metadata
// ---------------------------------------------------------------------------

const SEARCH_RESPONSE: ProtoTemplate = ProtoTemplate {
    name: "Search Response",
    description: "Search results with URLs, titles, and snippets",
    schema: r#"syntax = "proto3";

message SearchResult {
  string url = 1;
  string title = 2;
  string snippet = 3;
  int32 rank = 4;
  double score = 5;
}

message SearchResponse {
  string query = 1;
  int32 total_results = 2;
  repeated SearchResult results = 3;
  bool has_more = 4;
}"#,
    // query="protobuf", total_results=2,
    // result1: url="https://pb.dev", title="Docs", snippet="Guide", rank=1, score=0.95
    // result2: url="https://gh.com", title="Code", snippet="Source", rank=2, score=0.87
    // has_more=true
    //
    // field 1 string "protobuf":   0a 08 70 72 6f 74 6f 62 75 66
    // field 2 varint 2:            10 02
    // field 3 result1:             1a 28 (40 bytes)
    //   url "https://pb.dev":        0a 0e 68 74 74 70 73 3a 2f 2f 70 62 2e 64 65 76
    //   title "Docs":                12 04 44 6f 63 73
    //   snippet "Guide":             1a 05 47 75 69 64 65
    //   rank 1:                      20 01
    //   score 0.95:                  29 66 66 66 66 66 66 ee 3f
    // field 3 result2:             1a 29 (41 bytes)
    //   url "https://gh.com":        0a 0e 68 74 74 70 73 3a 2f 2f 67 68 2e 63 6f 6d
    //   title "Code":                12 04 43 6f 64 65
    //   snippet "Source":             1a 06 53 6f 75 72 63 65
    //   rank 2:                      20 02
    //   score 0.87:                  29 d7 a3 70 3d 0a d7 eb 3f
    // field 4 bool true:           20 01
    hex_data: "0a 08 70 72 6f 74 6f 62 75 66 10 02 1a 28 0a 0e 68 74 74 70 73 3a 2f 2f 70 62 2e 64 65 76 12 04 44 6f 63 73 1a 05 47 75 69 64 65 20 01 29 66 66 66 66 66 66 ee 3f 1a 29 0a 0e 68 74 74 70 73 3a 2f 2f 67 68 2e 63 6f 6d 12 04 43 6f 64 65 1a 06 53 6f 75 72 63 65 20 02 29 d7 a3 70 3d 0a d7 eb 3f 20 01",
};

// ---------------------------------------------------------------------------
// Template 4: Order with line items
// Realistic e-commerce order
// ---------------------------------------------------------------------------

const ORDER: ProtoTemplate = ProtoTemplate {
    name: "Order",
    description: "E-commerce order with line items and totals",
    schema: r#"syntax = "proto3";

message LineItem {
  string product_name = 1;
  int32 quantity = 2;
  int32 price_cents = 3;
}

message Address {
  string street = 1;
  string city = 2;
  string zip = 3;
}

message Order {
  string order_id = 1;
  int32 customer_id = 2;
  repeated LineItem items = 3;
  Address shipping = 4;
  int32 total_cents = 5;
  bool paid = 6;
}"#,
    // order_id="ORD-42", customer_id=1001
    // item1: product_name="Widget", quantity=3, price_cents=1500
    // item2: product_name="Gadget", quantity=1, price_cents=2999
    // shipping: street="123 Main St", city="Springfield", zip="62701"
    // total_cents=7499, paid=true
    //
    // field 1 "ORD-42":        0a 06 4f 52 44 2d 34 32
    // field 2 varint 1001:     10 e9 07
    // field 3 item1:           1a 0d (13 bytes)
    //   "Widget":                0a 06 57 69 64 67 65 74
    //   quantity 3:              10 03
    //   price_cents 1500:        18 dc 0b
    // field 3 item2:           1a 0d (13 bytes)
    //   "Gadget":                0a 06 47 61 64 67 65 74
    //   quantity 1:              10 01
    //   price_cents 2999:        18 b7 17
    // field 4 shipping:        22 21 (33 bytes)
    //   street "123 Main St":    0a 0b 31 32 33 20 4d 61 69 6e 20 53 74
    //   city "Springfield":      12 0b 53 70 72 69 6e 67 66 69 65 6c 64
    //   zip "62701":             1a 05 36 32 37 30 31
    // field 5 total_cents 7499: 28 cb 3a
    // field 6 paid true:       30 01
    hex_data: "0a 06 4f 52 44 2d 34 32 10 e9 07 1a 0d 0a 06 57 69 64 67 65 74 10 03 18 dc 0b 1a 0d 0a 06 47 61 64 67 65 74 10 01 18 b7 17 22 21 0a 0b 31 32 33 20 4d 61 69 6e 20 53 74 12 0b 53 70 72 69 6e 67 66 69 65 6c 64 1a 05 36 32 37 30 31 28 cb 3a 30 01",
};

// ---------------------------------------------------------------------------
// Template 5: Sensor Reading (fixed-width types)
// Shows double, float, fixed32, fixed64
// ---------------------------------------------------------------------------

const SENSOR_READING: ProtoTemplate = ProtoTemplate {
    name: "Sensor Data",
    description: "IoT sensor: timestamps, GPS coords, readings",
    schema: r#"syntax = "proto3";

message GpsCoord {
  double latitude = 1;
  double longitude = 2;
}

message SensorReading {
  fixed64 device_id = 1;
  fixed32 timestamp = 2;
  string sensor_type = 3;
  float temperature = 4;
  float humidity = 5;
  GpsCoord location = 6;
  bool alert = 7;
}"#,
    // device_id=0x00000000DEADBEEF, timestamp=1709856000
    // sensor_type="DHT22", temperature=23.5, humidity=65.2
    // location={lat=37.7749, lon=-122.4194}, alert=false
    //
    // field 1 fixed64:         09 ef be ad de 00 00 00 00
    // field 2 fixed32:         15 00 55 ea 65
    // field 3 string "DHT22":  1a 05 44 48 54 32 32
    // field 4 float 23.5:      25 00 00 bc 41
    // field 5 float 65.2:      2d 66 66 82 42
    // field 6 location:        32 12
    //   lat 37.7749:             09 d0 d5 56 ec 2f e3 42 40
    //   lon -122.4194:           11 50 fc 18 73 d7 9a 5e c0
    // field 7 bool false:      38 00
    hex_data: "09 ef be ad de 00 00 00 00 15 00 55 ea 65 1a 05 44 48 54 32 32 25 00 00 bc 41 2d 66 66 82 42 32 12 09 d0 d5 56 ec 2f e3 42 40 11 50 fc 18 73 d7 9a 5e c0 38 00",
};

// ---------------------------------------------------------------------------
// Template 6: Nested Config (3 levels deep)
// ---------------------------------------------------------------------------

const NESTED_CONFIG: ProtoTemplate = ProtoTemplate {
    name: "Nested Config",
    description: "3-level nested config: server > database > pool",
    schema: r#"syntax = "proto3";

message PoolConfig {
  int32 min_connections = 1;
  int32 max_connections = 2;
  int32 idle_timeout_ms = 3;
}

message DatabaseConfig {
  string host = 1;
  int32 port = 2;
  string db_name = 3;
  PoolConfig pool = 4;
}

message ServerConfig {
  string name = 1;
  int32 port = 2;
  bool debug = 3;
  DatabaseConfig database = 4;
  repeated string allowed_origins = 5;
}"#,
    // name="api-server", port=8080, debug=true
    // database: host="db.local", port=5432, db_name="myapp",
    //   pool: min=5, max=20, idle_timeout=30000
    // allowed_origins: ["https://app.com", "https://admin.com"]
    //
    // field 1 "api-server":       0a 0a 61 70 69 2d 73 65 72 76 65 72
    // field 2 varint 8080:        10 90 3f
    // field 3 bool true:          18 01
    // field 4 database:           22 1e (30 bytes)
    //   host "db.local":            0a 08 64 62 2e 6c 6f 63 61 6c
    //   port 5432:                  10 b8 2a
    //   db_name "myapp":            1a 05 6d 79 61 70 70
    //   pool:                       22 08 (8 bytes)
    //     min 5:                      08 05
    //     max 20:                     10 14
    //     idle_timeout 30000:         18 b0 ea 01
    // field 5 "https://app.com":  2a 0f 68 74 74 70 73 3a 2f 2f 61 70 70 2e 63 6f 6d
    // field 5 "https://admin.com":2a 11 68 74 74 70 73 3a 2f 2f 61 64 6d 69 6e 2e 63 6f 6d
    hex_data: "0a 0a 61 70 69 2d 73 65 72 76 65 72 10 90 3f 18 01 22 1e 0a 08 64 62 2e 6c 6f 63 61 6c 10 b8 2a 1a 05 6d 79 61 70 70 22 08 08 05 10 14 18 b0 ea 01 2a 0f 68 74 74 70 73 3a 2f 2f 61 70 70 2e 63 6f 6d 2a 11 68 74 74 70 73 3a 2f 2f 61 64 6d 69 6e 2e 63 6f 6d",
};

// ---------------------------------------------------------------------------
// Template 7: Mixed Wire Types
// All wire types in one message
// ---------------------------------------------------------------------------

const MIXED_WIRE_TYPES: ProtoTemplate = ProtoTemplate {
    name: "Mixed Wire Types",
    description: "All protobuf wire types in one message",
    schema: r#"syntax = "proto3";

message Example {
  int32 count = 1;
  fixed32 tag = 2;
  fixed64 big_id = 3;
  string name = 4;
  double ratio = 5;
  bool active = 6;
  bytes payload = 7;
}"#,
    // count=42, tag=0x12345678, big_id=0x0102030405060708,
    // name="hello", ratio=3.14, active=true, payload=<de ad be ef>
    //
    // field 1 varint 42:         08 2a
    // field 2 fixed32:           15 78 56 34 12
    // field 3 fixed64:           19 08 07 06 05 04 03 02 01
    // field 4 string "hello":    22 05 68 65 6c 6c 6f
    // field 5 double 3.14:       29 1f 85 eb 51 b8 1e 09 40
    // field 6 bool true:         30 01
    // field 7 bytes <deadbeef>:  3a 04 de ad be ef
    hex_data: "08 2a 15 78 56 34 12 19 08 07 06 05 04 03 02 01 22 05 68 65 6c 6c 6f 29 1f 85 eb 51 b8 1e 09 40 30 01 3a 04 de ad be ef",
};

// ---------------------------------------------------------------------------
// Template 8: Packed Repeated
// packed varint array
// ---------------------------------------------------------------------------

const PACKED_REPEATED: ProtoTemplate = ProtoTemplate {
    name: "Packed Repeated",
    description: "Packed repeated varint field [3, 270, 86942]",
    schema: r#"syntax = "proto3";

message Example {
  repeated int32 values = 1 [packed = true];
}
// values = [3, 270, 86942]"#,
    // field 1 length-delimited (packed): 0a 06 03 8e 02 9e a7 05
    hex_data: "0a 06 03 8e 02 9e a7 05",
};
