#ifndef ublox_ubx_parser_H
#define ublox_ubx_parser_H

#include "u-blox-bg/include/ubx/ubx.h"
#include <stdint.h>

#ifndef FFI_PLUGIN_EXPORT
#define FFI_PLUGIN_EXPORT
#endif

typedef struct {
  uint16_t len;
  uint8_t *data;
} ubx_packed_message;

typedef struct {
  uint32_t iTOW;
  uint16_t year;
  uint8_t month;
  uint8_t day;
  uint8_t hour;
  uint8_t min;
  uint8_t sec;
  uint8_t valid;
  uint32_t tAcc;
  int32_t nano;
  uint8_t fixType;
  uint8_t flags;
  uint8_t flags2;
  uint8_t numSV;
  int32_t lon;
  int32_t lat;
  int32_t height;
  int32_t hMSL;
  uint32_t hAcc;
  uint32_t vAcc;
  int32_t velN;
  int32_t velE;
  int32_t velD;
  int32_t gSpeed;
  int32_t headMot;
  uint32_t sAcc;
  uint32_t headAcc;
  uint16_t pDOP;
  uint16_t flags3;
  int32_t headVeh;
  int16_t magDec;
  uint16_t magAcc;
  char fixTypeString[32];
} ubx_pvt_data_t;

typedef struct {
  uint32_t iTOW;
  uint32_t dur;
  int32_t meanX;
  int32_t meanY;
  int32_t meanZ;
  int8_t meanXHP;
  int8_t meanYHP;
  int8_t meanZHP;
  uint32_t meanAcc;
  uint32_t obs;
  uint8_t valid;
  uint8_t active;
} ubx_svin_data_t;

FFI_PLUGIN_EXPORT void ublox_ubx_parser_init(void);

FFI_PLUGIN_EXPORT uint8_t ubx_parse(uint8_t byte);

FFI_PLUGIN_EXPORT uint8_t ubx_is_message_ready(void);

FFI_PLUGIN_EXPORT uint8_t ubx_get_last_message_type(void);

FFI_PLUGIN_EXPORT ubx_packed_message *ubx_get_raw_message(void);

FFI_PLUGIN_EXPORT ubx_pvt_data_t *ubx_get_pvt_data(void);

FFI_PLUGIN_EXPORT ubx_svin_data_t *ubx_get_svin_data(void);

FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_svin_min_dur(uint32_t seconds);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_acc_min(double accuracy_meters);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_mode(void);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_mode_disabled(void);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_mode_fixed(void);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_pos_type_llh(void);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lat(int32_t lat);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lon(int32_t lon);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_height(int32_t height_cm);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lat_hp(int8_t lat_hp);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_lon_hp(int8_t lon_hp);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_tmode_height_hp(int8_t height_hp);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_fixed_pos_acc(uint32_t accuracy_0_1_mm);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_valdel_all(void);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_restart(void);
FFI_PLUGIN_EXPORT ubx_packed_message ubx_pack_valset(uint32_t key, uint8_t value_size, uint32_t value);

#endif
