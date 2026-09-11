/* Synthetic V.17 long/short training and V.29/V.27ter phase sweeps.
 * No captured fax data is needed to expose the signed phase-wrap regression. */
#define SPANDSP_EXPOSE_INTERNAL_STRUCTURES
#include "spandsp.h"
#include <stdio.h>

struct result
{
    int trained;
    int failed;
    int bits;
    int errors;
};

static int get_bit(void *user_data)
{
    (void) user_data;
    return 0;
}

static void put_bit(void *user_data, int bit)
{
    struct result *result = user_data;
    if (bit == SIG_STATUS_TRAINING_SUCCEEDED)
        result->trained++;
    else if (bit == SIG_STATUS_TRAINING_FAILED)
        result->failed++;
    else if (bit >= 0)
    {
        /* Allow the decoder's initial traceback to settle. */
        if (result->bits++ >= 100 && bit != 0)
            result->errors++;
    }
}

static int check(const struct result *result, int rate, int phase, const char *modem, const char *training)
{
    if (result->trained == 1 && result->failed == 0 && result->bits > 1000 && result->errors == 0)
        return 0;
    fprintf(stderr, "%s %d bps, phase %d/16, %s training: trained=%d failed=%d bits=%d errors=%d\n",
            modem, rate, phase, training, result->trained, result->failed, result->bits, result->errors);
    return 1;
}

int main(void)
{
    int failures = 0;
    for (int rate = 7200; rate <= 14400; rate += 2400)
    {
        for (int phase = 0; phase < 16; phase++)
        {
            struct result result = {0};
            v17_tx_state_t *tx = v17_tx_init(NULL, rate, false, get_bit, NULL);
            v17_rx_state_t *rx = v17_rx_init(NULL, rate, put_bit, &result);
            int16_t samples[160];
            if (!tx || !rx)
                return 1;
            for (int block = 0; block < 150; block++)
            {
                v17_tx(tx, samples, 160);
                v17_rx(rx, samples, 160);
            }
            failures += check(&result, rate, phase, "V.17", "long");
            if (v17_tx_restart(tx, rate, false, true) || v17_rx_restart(rx, rate, true))
                return 1;
            tx->carrier_phase = (uint32_t) phase * UINT32_C(0x10000000);
            result = (struct result) {0};
            for (int block = 0; block < 100; block++)
            {
                v17_tx(tx, samples, 160);
                v17_rx(rx, samples, 160);
            }
            failures += check(&result, rate, phase, "V.17", "short");
            v17_tx_free(tx);
            v17_rx_free(rx);
        }
    }
    for (int rate = 4800; rate <= 9600; rate += 2400)
    {
        for (int phase = 0; phase < 16; phase++)
        {
            struct result result = {0};
            v29_tx_state_t *tx = v29_tx_init(NULL, rate, false, get_bit, NULL);
            v29_rx_state_t *rx = v29_rx_init(NULL, rate, put_bit, &result);
            int16_t samples[160];
            if (!tx || !rx)
                return 1;
            tx->carrier_phase = (uint32_t) phase * UINT32_C(0x10000000);
            for (int block = 0; block < 150; block++)
            {
                v29_tx(tx, samples, 160);
                v29_rx(rx, samples, 160);
            }
            failures += check(&result, rate, phase, "V.29", "long");
            v29_tx_free(tx);
            v29_rx_free(rx);
        }
    }
    for (int rate = 2400; rate <= 4800; rate += 2400)
    {
        for (int phase = 0; phase < 16; phase++)
        {
            struct result result = {0};
            v27ter_tx_state_t *tx = v27ter_tx_init(NULL, rate, false, get_bit, NULL);
            v27ter_rx_state_t *rx = v27ter_rx_init(NULL, rate, put_bit, &result);
            int16_t samples[160];
            if (!tx || !rx)
                return 1;
            tx->carrier_phase = (uint32_t) phase * UINT32_C(0x10000000);
            for (int block = 0; block < 150; block++)
            {
                v27ter_tx(tx, samples, 160);
                v27ter_rx(rx, samples, 160);
            }
            failures += check(&result, rate, phase, "V.27ter", "long");
            v27ter_tx_free(tx);
            v27ter_rx_free(rx);
        }
    }
    return failures != 0;
}
