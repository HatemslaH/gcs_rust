#include "ublox_ubx_parser.h"
#include "u-blox-bg/include/ubx/ubx-cfg-rst.h"
#include "u-blox-bg/include/ubx/ubx-cfg-valdel.h"
#include "u-blox-bg/include/ubx/ubx-cfg-valset.h"
#include "u-blox-bg/include/ubx/ubx-enum.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>

// Структуры для передачи данных в D

// Структура для хранения состояния парсера
typedef struct {
  ubx_default_msg_t rx_msg;
  ubx_read_track_t rx_track;
  ubx_packed_message readed_msg;
  uint8_t buffer[1024];
  uint8_t msg_ready;
  ubx_pvt_data_t pvt_data;
  ubx_svin_data_t svin_data;
  uint8_t last_msg_type; // 1 = PVT, 2 = SVIN, 3 = ACK_ACK, 4 = ACK_NAK

  uint8_t parsing_in_progress; // Флаг, что мы в процессе сбора сообщения
  uint16_t bytes_parsed; // Сколько байт уже обработано в текущем сообщении
} ublox_ubx_parser_state_t;

static ublox_ubx_parser_state_t parser_state;

FFI_PLUGIN_EXPORT uint8_t ubx_parse(uint8_t byte) {
  ubx_nav_pvt_t rx_pvt;
  ubx_nav_svin_t rx_svin;

  // Сбрасываем флаг готовности
  parser_state.msg_ready = 0;
  parser_state.last_msg_type = 0;

  if (ubx_parse_char(byte, &parser_state.rx_msg, &parser_state.rx_track)) {
    switch (parser_state.rx_msg.header) {
    case UBX_NAV_PVT:
      ubx_default_msg_t2ubx_nav_pvt(&parser_state.rx_msg, &rx_pvt);

      // Копируем сырые данные в буфер
      parser_state.readed_msg.len = parser_state.rx_msg.length;
      memcpy(parser_state.buffer, parser_state.rx_msg.payload,
             parser_state.rx_msg.length);
      parser_state.readed_msg.data = parser_state.buffer;

      // Заполняем структуру PVT для Dart
      parser_state.pvt_data.iTOW = rx_pvt.iTOW;
      parser_state.pvt_data.year = rx_pvt.year;
      parser_state.pvt_data.month = rx_pvt.month;
      parser_state.pvt_data.day = rx_pvt.day;
      parser_state.pvt_data.hour = rx_pvt.hour;
      parser_state.pvt_data.min = rx_pvt.min;
      parser_state.pvt_data.sec = rx_pvt.sec;
      parser_state.pvt_data.valid = rx_pvt.valid;
      parser_state.pvt_data.tAcc = rx_pvt.tAcc;
      parser_state.pvt_data.nano = rx_pvt.nano;
      parser_state.pvt_data.fixType = rx_pvt.fixType;
      parser_state.pvt_data.flags = rx_pvt.flags;
      parser_state.pvt_data.flags2 = rx_pvt.flags2;
      parser_state.pvt_data.numSV = rx_pvt.numSV;
      parser_state.pvt_data.lon = rx_pvt.lon;
      parser_state.pvt_data.lat = rx_pvt.lat;
      parser_state.pvt_data.height = rx_pvt.height;
      parser_state.pvt_data.hMSL = rx_pvt.hMSL;
      parser_state.pvt_data.hAcc = rx_pvt.hAcc;
      parser_state.pvt_data.vAcc = rx_pvt.vAcc;
      parser_state.pvt_data.velN = rx_pvt.velN;
      parser_state.pvt_data.velE = rx_pvt.velE;
      parser_state.pvt_data.velD = rx_pvt.velD;
      parser_state.pvt_data.gSpeed = rx_pvt.gSpeed;
      parser_state.pvt_data.headMot = rx_pvt.headMot;
      parser_state.pvt_data.sAcc = rx_pvt.sAcc;
      parser_state.pvt_data.headAcc = rx_pvt.headAcc;
      parser_state.pvt_data.pDOP = rx_pvt.pDOP;
      parser_state.pvt_data.flags3 = rx_pvt.flags3;
      parser_state.pvt_data.headVeh = rx_pvt.headVeh;
      parser_state.pvt_data.magDec = rx_pvt.magDec;
      parser_state.pvt_data.magAcc = rx_pvt.magAcc;

      // Заполняем строку с типом фикса
      switch (rx_pvt.fixType) {
      case UBX_NAV_PVT_FIXTYPE_NOFIX:
        strcpy(parser_state.pvt_data.fixTypeString, "No Fix");
        break;
      case UBX_NAV_PVT_FIXTYPE_DR:
        strcpy(parser_state.pvt_data.fixTypeString, "Dead Reckoning");
        break;
      case UBX_NAV_PVT_FIXTYPE_2D:
        strcpy(parser_state.pvt_data.fixTypeString, "2D Fix");
        break;
      case UBX_NAV_PVT_FIXTYPE_3D:
        strcpy(parser_state.pvt_data.fixTypeString, "3D Fix");
        break;
      case UBX_NAV_PVT_FIXTYPE_GNSS_DR:
        strcpy(parser_state.pvt_data.fixTypeString, "GNSS+DR");
        break;
      case UBX_NAV_PVT_FIXTYPE_TIME:
        strcpy(parser_state.pvt_data.fixTypeString, "TIME");
        break;
      default:
        strcpy(parser_state.pvt_data.fixTypeString, "Unknown");
        break;
      }

      parser_state.msg_ready = 1;
      parser_state.last_msg_type = 1; // PVT

      // Логирование в консоль C
      // printf(
      //     "[C] UBX_NAV_PVT: %s, Sats: %d, Lat: %.6f, Lon: %.6f, Alt: %.2f
      //     m\n", parser_state.pvt_data.fixTypeString, rx_pvt.numSV, rx_pvt.lat
      //     / 10000000.0, rx_pvt.lon / 10000000.0, rx_pvt.hMSL / 1000.0);

      return 0;

    case UBX_NAV_SVIN:
      ubx_default_msg_t2ubx_nav_svin(&parser_state.rx_msg, &rx_svin);

      // Копируем сырые данные в буфер
      parser_state.readed_msg.len = parser_state.rx_msg.length;
      memcpy(parser_state.buffer, parser_state.rx_msg.payload,
             parser_state.rx_msg.length);
      parser_state.readed_msg.data = parser_state.buffer;

      // Заполняем структуру SVIN для Dart
      parser_state.svin_data.iTOW = rx_svin.iTOW;
      parser_state.svin_data.dur = rx_svin.dur;
      parser_state.svin_data.meanX = rx_svin.meanX;
      parser_state.svin_data.meanY = rx_svin.meanY;
      parser_state.svin_data.meanZ = rx_svin.meanZ;
      parser_state.svin_data.meanXHP = rx_svin.meanXHP;
      parser_state.svin_data.meanYHP = rx_svin.meanYHP;
      parser_state.svin_data.meanZHP = rx_svin.meanZHP;
      parser_state.svin_data.meanAcc = rx_svin.meanAcc;
      parser_state.svin_data.obs = rx_svin.obs;
      parser_state.svin_data.valid = rx_svin.valid;
      parser_state.svin_data.active = rx_svin.active;

      parser_state.msg_ready = 1;
      parser_state.last_msg_type = 2; // SVIN

      // Логирование в консоль C
      // printf("[C] UBX_NAV_SVIN: active=%u, valid=%u, dur=%u s, meanAcc=%.3f
      // m, "
      //        "obs=%u\n",
      //        rx_svin.active, rx_svin.valid, rx_svin.dur,
      //        rx_svin.meanAcc / 10000.0, rx_svin.obs);

      return 0;

    case UBX_ACK_ACK:
      // printf("[C] GET UBX_ACK_ACK\n");

      parser_state.readed_msg.len = parser_state.rx_msg.length;
      memcpy(parser_state.buffer, parser_state.rx_msg.payload,
             parser_state.rx_msg.length);
      parser_state.readed_msg.data = parser_state.buffer;
      parser_state.msg_ready = 1;
      parser_state.last_msg_type = 3; // ACK_ACK

      return 0;

    case UBX_ACK_NAK:
      // printf("[C] GET UBX_ACK_NAK\n");

      parser_state.readed_msg.len = parser_state.rx_msg.length;
      memcpy(parser_state.buffer, parser_state.rx_msg.payload,
             parser_state.rx_msg.length);
      parser_state.readed_msg.data = parser_state.buffer;
      parser_state.msg_ready = 1;
      parser_state.last_msg_type = 4; // ACK_NAK

      return 0;
    }
  }

  return 1;
}

FFI_PLUGIN_EXPORT uint8_t ubx_is_message_ready() {
  return parser_state.msg_ready;
}

FFI_PLUGIN_EXPORT uint8_t ubx_get_last_message_type() {
  return parser_state.last_msg_type;
}

FFI_PLUGIN_EXPORT ubx_packed_message *ubx_get_raw_message() {
  if (parser_state.msg_ready) {
    return &parser_state.readed_msg;
  }
  return NULL;
}

FFI_PLUGIN_EXPORT ubx_pvt_data_t *ubx_get_pvt_data() {
  if (parser_state.msg_ready && parser_state.last_msg_type == 1) {
    return &parser_state.pvt_data;
  }
  return NULL;
}

FFI_PLUGIN_EXPORT ubx_svin_data_t *ubx_get_svin_data() {
  if (parser_state.msg_ready && parser_state.last_msg_type == 2) {
    return &parser_state.svin_data;
  }
  return NULL;
}

FFI_PLUGIN_EXPORT void ublox_ubx_parser_init() {
  memset(&parser_state, 0, sizeof(parser_state));
}

FFI_PLUGIN_EXPORT ubx_packed_message
ubx_pack_tmode_svin_min_dur(uint32_t seconds) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_SVIN_MIN_DUR;
  valset.cfgData_value_size = sizeof(uint32_t);
  valset.cfgData_value = seconds;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_acc_min(double accuracy_meters) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_SVIN_ACC_LIMIT;
  valset.cfgData_value_size = sizeof(uint32_t);
  valset.cfgData_value = (uint32_t)(accuracy_meters * 10000.0); // 0.1 mm units

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_mode() {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_MODE;
  valset.cfgData_value_size = sizeof(uint8_t);
  valset.cfgData_value = UBX_CFG_VALSET_TMODE_MODE_SURVEY_IN;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_mode_disabled() {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_MODE;
  valset.cfgData_value_size = sizeof(uint8_t);
  valset.cfgData_value = UBX_CFG_VALSET_TMODE_MODE_DISABLED;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_mode_fixed() {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_MODE;
  valset.cfgData_value_size = sizeof(uint8_t);
  valset.cfgData_value = UBX_CFG_VALSET_TMODE_MODE_FIXED;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_pos_type_llh() {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_POS_TYPE;
  valset.cfgData_value_size = sizeof(uint8_t);
  valset.cfgData_value = UBX_CFG_VALSET_TMODE_POS_TYPE_LLH;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lat(int32_t lat) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_LAT;
  valset.cfgData_value_size = sizeof(int32_t);
  // Битовый образ I4 для memcpy в VALSET
  valset.cfgData_value = (uint32_t)lat;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lon(int32_t lon) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_LON;
  valset.cfgData_value_size = sizeof(int32_t);
  valset.cfgData_value = (uint32_t)lon;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_height(int32_t height_cm) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_HEIGHT;
  valset.cfgData_value_size = sizeof(int32_t);
  valset.cfgData_value = (uint32_t)height_cm;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lat_hp(int8_t lat_hp) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_LAT_HP;
  valset.cfgData_value_size = sizeof(int8_t);
  // Младший байт LE — знаковый I1
  valset.cfgData_value = (uint32_t)(int32_t)lat_hp;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lon_hp(int8_t lon_hp) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_LON_HP;
  valset.cfgData_value_size = sizeof(int8_t);
  valset.cfgData_value = (uint32_t)(int32_t)lon_hp;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_height_hp(int8_t height_hp) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_HEIGHT_HP;
  valset.cfgData_value_size = sizeof(int8_t);
  valset.cfgData_value = (uint32_t)(int32_t)height_hp;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_fixed_pos_acc(uint32_t accuracy_0_1_mm) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = UBX_KEY_CFG_TMODE_FIXED_POS_ACC;
  valset.cfgData_value_size = sizeof(uint32_t);
  valset.cfgData_value = accuracy_0_1_mm;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_valdel_all() {
  ubx_cfg_valdel_t valdel;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valdel.version = 0x00;
  valdel.layers = UBX_CFG_VADEL_LAYERS_BBR | UBX_CFG_VALDEL_LAYERS_FLASH;
  valdel.key = UBX_CFG_KEY_ALL;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valdel2array(&valdel, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_restart() {
  ubx_cfg_rst_t rst;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  rst.navBbrMask = UBX_CFG_RST_NAVBBRMASK_HOT_START;
  rst.resetMode = UBX_CFG_RST_RESETMODE_SW;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_rst2array(&rst, msg_buffer);

  return msg;
}

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_valset(uint32_t key,
                                                     uint8_t value_size,
                                                     uint32_t value) {
  ubx_cfg_valset_t valset;
  ubx_packed_message msg;
  static uint8_t msg_buffer[256];

  valset.version = 0x00;
  valset.layers = UBX_CFG_VALSET_LAYERS_RAM;
  valset.cfgData_key = key;
  valset.cfgData_value_size = value_size;
  valset.cfgData_value = value;

  msg.data = msg_buffer;
  msg.len = ubx_cfg_valset2array(&valset, msg_buffer);

  return msg;
}
