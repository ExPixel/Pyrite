#ifndef ma_lpf_h
#define ma_lpf_h

/*
TODO:
  - Document passthrough behaviour of the biquad filter and how it doesn't update previous inputs and outputs.
  - Document how changing biquad constants requires reinitialization of the filter (due to issue above). ma_biquad_reinit().
  - Document how ma_biquad_process() and ma_lpf_process() supports in-place filtering by passing in the same buffer for both the input and output.
*/

typedef struct
{
    ma_format format;
    ma_uint32 channels;
    double a0;
    double a1;
    double a2;
    double b0;
    double b1;
    double b2;
} ma_biquad_config;

ma_biquad_config ma_biquad_config_init(ma_format format, ma_uint32 channels, double a0, double a1, double a2, double b0, double b1, double b2);

typedef struct
{
    ma_biquad_config config;
    ma_bool32 isPassthrough;
    ma_uint32 prevFrameCount;
    float x1[MA_MAX_CHANNELS];   /* x[n-1] */
    float x2[MA_MAX_CHANNELS];   /* x[n-2] */
    float y1[MA_MAX_CHANNELS];   /* y[n-1] */
    float y2[MA_MAX_CHANNELS];   /* y[n-2] */
} ma_biquad;

ma_result ma_biquad_init(const ma_biquad_config* pConfig, ma_biquad* pBQ);
ma_result ma_biquad_reinit(const ma_biquad_config* pConfig, ma_biquad* pBQ);
ma_result ma_biquad_process(ma_biquad* pBQ, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount);


typedef struct
{
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    ma_uint32 cutoffFrequency;
} ma_lpf_config;

ma_lpf_config ma_lpf_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, ma_uint32 cutoffFrequency);

typedef struct
{
    ma_biquad bq;   /* The low-pass filter is implemented as a biquad filter. */
    ma_lpf_config config;
} ma_lpf;

ma_result ma_lpf_init(const ma_lpf_config* pConfig, ma_lpf* pLPF);
ma_result ma_lpf_reinit(const ma_lpf_config* pConfig, ma_lpf* pLPF);
ma_result ma_lpf_process(ma_lpf* pLPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount);

#endif  /* ma_lpf_h */




#if defined(MINIAUDIO_IMPLEMENTATION)

ma_biquad_config ma_biquad_config_init(ma_format format, ma_uint32 channels, double a0, double a1, double a2, double b0, double b1, double b2)
{
    ma_biquad_config config;

    MA_ZERO_OBJECT(&config);
    config.format = format;
    config.channels = channels;
    config.a0 = a0;
    config.a1 = a1;
    config.a2 = a2;
    config.b0 = b0;
    config.b1 = b1;
    config.b2 = b2;

    return config;
}

ma_result ma_biquad_init(const ma_biquad_config* pConfig, ma_biquad* pBQ)
{
    if (pBQ == NULL) {
        return MA_INVALID_ARGS;
    }

    MA_ZERO_OBJECT(pBQ);

    if (pConfig == NULL) {
        return MA_INVALID_ARGS;
    }

    return ma_biquad_reinit(pConfig, pBQ);
}

ma_result ma_biquad_reinit(const ma_biquad_config* pConfig, ma_biquad* pBQ)
{
    if (pBQ == NULL || pConfig == NULL) {
        return MA_INVALID_ARGS;
    }

    if (pConfig->a0 == 0) {
        return MA_INVALID_ARGS; /* Division by zero. */
    }

    /* Currently only supporting f32 and s16, but support for other formats will be added later. */
    if (pConfig->format != ma_format_f32 && pConfig->format != ma_format_s16) {
        return MA_INVALID_ARGS;
    }

    pBQ->config = *pConfig;

    if (pConfig->a0 == 1 && pConfig->a1 == 0 && pConfig->a2 == 0 &&
        pConfig->b0 == 1 && pConfig->b1 == 0 && pConfig->b2 == 0) {
        pBQ->isPassthrough = MA_TRUE;
    }

    /* Normalize. */
    pBQ->config.a1 /= pBQ->config.a0;
    pBQ->config.a2 /= pBQ->config.a0;
    pBQ->config.b0 /= pBQ->config.a0;
    pBQ->config.b1 /= pBQ->config.a0;
    pBQ->config.b2 /= pBQ->config.a0;

    return MA_SUCCESS;
}

ma_result ma_biquad_process(ma_biquad* pBQ, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
{
    ma_uint32 n;
    ma_uint32 c;
    double a1 = pBQ->config.a1;
    double a2 = pBQ->config.a2;
    double b0 = pBQ->config.b0;
    double b1 = pBQ->config.b1;
    double b2 = pBQ->config.b2;

    if (pBQ == NULL || pFramesOut == NULL || pFramesIn == NULL) {
        return MA_INVALID_ARGS;
    }

    /* Fast path for passthrough. */
    if (pBQ->isPassthrough) {
        if (pFramesOut != pFramesIn) {  /* <-- The output buffer is allowed to be the same as the input buffer. */
            ma_copy_memory_64(pFramesOut, pFramesIn, frameCount * ma_get_bytes_per_frame(pBQ->config.format, pBQ->config.channels));
        }

        return MA_SUCCESS;
    }

    /* Note that the logic below needs to support in-place filtering. That is, it must support the case where pFramesOut and pFramesIn are the same. */

    /* Currently only supporting f32. */
    if (pBQ->config.format == ma_format_f32) {
              float* pY = (      float*)pFramesOut;
        const float* pX = (const float*)pFramesIn;

        for (n = 0; n < frameCount; n += 1) {
            for (c = 0; c < pBQ->config.channels; c += 1) {
                double x2 = pBQ->x2[c];
                double x1 = pBQ->x1[c];
                double x0 = pX[n*pBQ->config.channels + c];
                double y2 = pBQ->y2[c];
                double y1 = pBQ->y1[c];
                double y0 = b0*x0 + b1*x1 + b2*x2 - a1*y1 - a2*y2;
                
                pY[n*pBQ->config.channels + c] = (float)y0;
                pBQ->x2[c] = (float)x1;
                pBQ->x1[c] = (float)x0;
                pBQ->y2[c] = (float)y1;
                pBQ->y1[c] = (float)y0;
            }
        }
    } else if (pBQ->config.format == ma_format_s16) {
        /* */ ma_int16* pY = (      ma_int16*)pFramesOut;
        const ma_int16* pX = (const ma_int16*)pFramesIn;

        for (n = 0; n < frameCount; n += 1) {
            for (c = 0; c < pBQ->config.channels; c += 1) {
                double x2 = pBQ->x2[c];
                double x1 = pBQ->x1[c];
                double x0 = pX[n*pBQ->config.channels + c] * 0.000030517578125; /* s16 -> f32 */
                double y2 = pBQ->y2[c];
                double y1 = pBQ->y1[c];
                double y0 = b0*x0 + b1*x1 + b2*x2 - a1*y1 - a2*y2;
                
                pY[n*pBQ->config.channels + c] = (ma_int16)(y0 * 32767.0);      /* f32 -> s16 */
                pBQ->x2[c] = (float)x1;
                pBQ->x1[c] = (float)x0;
                pBQ->y2[c] = (float)y1;
                pBQ->y1[c] = (float)y0;
            }
        }
    } else {
        MA_ASSERT(MA_FALSE);
        return MA_INVALID_ARGS; /* Format not supported. Should never hit this because it's checked in ma_biquad_init(). */
    }

    return MA_SUCCESS;
}


ma_lpf_config ma_lpf_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, ma_uint32 cutoffFrequency)
{
    ma_lpf_config config;
    
    MA_ZERO_OBJECT(&config);
    config.format = format;
    config.channels = channels;
    config.sampleRate = sampleRate;
    config.cutoffFrequency = cutoffFrequency;

    return config;
}

static MA_INLINE ma_biquad_config ma_lpf__get_biquad_config(const ma_lpf_config* pConfig)
{
    ma_biquad_config bqConfig;
    double q;
    double w;
    double s;
    double c;
    double a;

    MA_ASSERT(pConfig != NULL);

    q = 1 / sqrt(2);
    w = 2 * MA_PI_D * pConfig->cutoffFrequency / pConfig->sampleRate;
    s = sin(w);
    c = cos(w);
    a = s / (2*q);

    bqConfig.a0 = (double)( 1 + a);
    bqConfig.a1 = (double)(-2 * c);
    bqConfig.a2 = (double)( 1 - a);
    bqConfig.b0 = (double)((1 - c) / 2);
    bqConfig.b1 = (double)( 1 - c);
    bqConfig.b2 = (double)((1 - c) / 2);

    bqConfig.format   = pConfig->format;
    bqConfig.channels = pConfig->channels;

    return bqConfig;
}

ma_result ma_lpf_init(const ma_lpf_config* pConfig, ma_lpf* pLPF)
{
    ma_result result;
    ma_biquad_config bqConfig;

    if (pLPF == NULL) {
        return MA_INVALID_ARGS;
    }

    MA_ZERO_OBJECT(pLPF);

    if (pConfig == NULL) {
        return MA_INVALID_ARGS;
    }

    pLPF->config = *pConfig;

    bqConfig = ma_lpf__get_biquad_config(pConfig);
    result = ma_biquad_init(&bqConfig, &pLPF->bq);
    if (result != MA_SUCCESS) {
        return result;
    }

    return MA_SUCCESS;
}

ma_result ma_lpf_reinit(const ma_lpf_config* pConfig, ma_lpf* pLPF)
{
    ma_result result;
    ma_biquad_config bqConfig;

    if (pLPF == NULL || pConfig == NULL) {
        return MA_INVALID_ARGS;
    }

    pLPF->config = *pConfig;

    bqConfig = ma_lpf__get_biquad_config(pConfig);
    result = ma_biquad_reinit(&bqConfig, &pLPF->bq);
    if (result != MA_SUCCESS) {
        return result;
    }

    return MA_SUCCESS;
}

ma_result ma_lpf_process(ma_lpf* pLPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
{
    if (pLPF == NULL) {
        return MA_INVALID_ARGS;
    }

    return ma_biquad_process(&pLPF->bq, pFramesOut, pFramesIn, frameCount);
}

#endif