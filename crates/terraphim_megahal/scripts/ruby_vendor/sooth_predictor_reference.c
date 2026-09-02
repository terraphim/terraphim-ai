//==============================================================================

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#include "sooth_predictor.h"

//------------------------------------------------------------------------------

SoothPredictor *
sooth_predictor_init()
{
  SoothPredictor * predictor = malloc(sizeof(SoothPredictor));

  if (predictor == NULL)
  {
    return NULL;
  }

  predictor->error_event = 0;
  predictor->contexts = NULL;
  predictor->contexts_size = 0;

  return predictor;
}

//------------------------------------------------------------------------------

void
sooth_predictor_clear(SoothPredictor * predictor)
{
  for (uint32_t i = 0; i < predictor->contexts_size; ++i)
  {
    SoothContext * context = &(predictor->contexts[i]);
    free(context->statistics);
  }

  free(predictor->contexts);
  predictor->contexts = NULL;
  predictor->contexts_size = 0;
}

//------------------------------------------------------------------------------

void
sooth_predictor_free(SoothPredictor * predictor)
{
  sooth_predictor_clear(predictor);
  free(predictor);
}

//------------------------------------------------------------------------------

bool
sooth_predictor_save(const char * const filename, SoothPredictor * predictor)
{
  FILE *file = fopen(filename, "wb");

  if (file == NULL)
  {
    return false;
  }

  fwrite("MH11", 1, 4, file);
  fwrite(&predictor->error_event, 4, 1, file);
  fwrite(&predictor->contexts_size, 4, 1, file);
  for (uint32_t i = 0; i < predictor->contexts_size; ++i)
  {
    SoothContext context = predictor->contexts[i];
    fwrite(&context.id, 4, 1, file);
    fwrite(&context.count, 4, 1, file);
    fwrite(&context.statistics_size, 4, 1, file);
    fwrite(context.statistics, sizeof(SoothStatistic), context.statistics_size, file);
  }

  fclose(file);

  return true;
}

//------------------------------------------------------------------------------

bool sooth_predictor_load(const char * const filename, SoothPredictor * predictor)
{
  FILE *file = fopen(filename, "rb");

  if (file == NULL)
  {
    return false;
  }

  char code[4];
  fread(code, 1, 4, file);
  if (strncmp(code, "MH11", 4) != 0)
  {
    return false;
  }

  sooth_predictor_clear(predictor);

  fread(&predictor->error_event, 4, 1, file);
  fread(&predictor->contexts_size, 4, 1, file);
  if (predictor->contexts_size == 0)
  {
    return true;
  }
  predictor->contexts = malloc(sizeof(SoothContext) * predictor->contexts_size);
  if (predictor->contexts == NULL)
  {
    sooth_predictor_clear(predictor);
    return false;
  }
  for (uint32_t i = 0; i < predictor->contexts_size; ++i)
  {
    SoothContext * context = &(predictor->contexts[i]);
    fread(&context->id, 4, 1, file);
    fread(&context->count, 4, 1, file);
    fread(&context->statistics_size, 4, 1, file);
    context->statistics = malloc(sizeof(SoothStatistic) * context->statistics_size);
    if (context->statistics == NULL)
    {
      sooth_predictor_clear(predictor);
      return false;
    }
    fread(context->statistics, sizeof(SoothStatistic), context->statistics_size, file);
  }

  return true;
}

//------------------------------------------------------------------------------
SoothContext *
sooth_predictor_find_context(SoothPredictor * predictor, uint32_t id)
{
  SoothContext * context = NULL;
  uint32_t mid = 0;

  if (predictor->contexts_size > 0)
  {
    uint32_t low = 0;
    uint32_t high = predictor->contexts_size - 1;

    while (low <= high)
    {
      mid = low + (high - low) / 2;
      context = &(predictor->contexts[mid]);
      if (context->id < id)
      {
        low = mid + 1;
      }
      else if (context->id > id)
      {
        if (mid == 0)
        {
          break;
        }
        high = mid - 1;
      }
      else
      {
        return context;
      }
    }

    mid = low;
  }

  predictor->contexts_size += 1;
  SoothContext * new_memory = realloc(predictor->contexts, sizeof(SoothContext) * predictor->contexts_size);
  if (new_memory == NULL)
  {
    return NULL;
  }
  predictor->contexts = new_memory;

  if ((mid + 1) < predictor->contexts_size)
  {
    SoothContext * src = &(predictor->contexts[mid]);
    SoothContext * dest = &(predictor->contexts[mid+1]);
    (void)memmove(dest, src, sizeof(SoothContext) * (predictor->contexts_size - mid - 1)); 
  }

  context = &(predictor->contexts[mid]);
  context->id = id;
  context->count = 0;
  context->statistics_size = 0;
  context->statistics = NULL;

  return context;
}

//------------------------------------------------------------------------------

SoothStatistic *
sooth_predictor_find_statistic(SoothContext * context, uint32_t event)
{
  SoothStatistic * statistic = NULL;
  uint32_t mid = 0;

  if (context->statistics_size > 0)
  {
    uint32_t low = 0;
    uint32_t high = context->statistics_size - 1;

    while (low <= high)
    {
      mid = low + (high - low) / 2;
      statistic = &(context->statistics[mid]);
      if (statistic->event < event)
      {
        low = mid + 1;
      }
      else if (statistic->event > event)
      {
        if (mid == 0)
        {
          break;
        }
        high = mid - 1;
      }
      else
      {
        return statistic;
      }
    }

    mid = low;
  }

  context->statistics_size += 1;
  SoothStatistic * new_memory = realloc(context->statistics, sizeof(SoothStatistic) * context->statistics_size);
  if (new_memory == NULL)
  {
    return NULL;
  }
  context->statistics = new_memory;

  if ((mid + 1) < context->statistics_size)
  {
    SoothStatistic * src = &(context->statistics[mid]);
    SoothStatistic * dest = &(context->statistics[mid+1]);
    (void)memmove(dest, src, sizeof(SoothStatistic) * (context->statistics_size - mid - 1)); 
  }

  statistic = &(context->statistics[mid]);
  statistic->event = event;
  statistic->count = 0;

  return statistic;
}

//------------------------------------------------------------------------------

uint32_t
sooth_predictor_size(SoothPredictor * predictor, uint32_t id)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);
  
  if (context == NULL)
  {
    return 0;
  }

  return context->statistics_size;
}

//------------------------------------------------------------------------------

uint32_t
sooth_predictor_count(SoothPredictor * predictor, uint32_t id)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);
  
  if (context == NULL)
  {
    return 0;
  }

  return context->count;
}

//------------------------------------------------------------------------------

uint32_t
sooth_predictor_observe(SoothPredictor * predictor, uint32_t id, uint32_t event)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);

  if (context == NULL)
  {
    return 0;
  }

  if (context->count == UINT32_MAX)
  {
    context->count = 0;
    for (uint32_t i = 0; i < context->statistics_size; ++i)
    {
      SoothStatistic statistic = context->statistics[i];
      statistic.count /= 2;
      context->count += statistic.count;
    }
  }

  SoothStatistic * statistic = sooth_predictor_find_statistic(context, event);

  if (statistic == NULL)
  {
    return 0;
  }

  statistic->count += 1;
  context->count += 1;

  return statistic->count;
}

//------------------------------------------------------------------------------

uint32_t
sooth_predictor_select(SoothPredictor * predictor, uint32_t id, uint32_t limit)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);

  if (context == NULL || limit == 0 || limit > context->count)
  {
    return predictor->error_event;
  }

  for (uint32_t i = 0; i < context->statistics_size; ++i)
  {
    SoothStatistic statistic = context->statistics[i];
    if (limit > statistic.count)
    {
      limit -= statistic.count;
      continue;
    }
    return statistic.event;
  }

  return predictor->error_event;
}

//------------------------------------------------------------------------------

SoothStatistic *
sooth_predictor_distribution(SoothPredictor * predictor, uint32_t id)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);

  if (context == NULL)
  {
    return NULL;
  }

  return context->statistics;
}

//------------------------------------------------------------------------------

double
sooth_predictor_uncertainty(SoothPredictor * predictor, uint32_t id)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);

  if (context == NULL || context->count == 0)
  {
    return -1;
  }

  double uncertainty = 0.0;
  for (uint32_t i = 0; i < context->statistics_size; ++i)
  {
    if (context->statistics[i].count > 0)
    {
      double frequency = (double)context->statistics[i].count / (double)context->count;
      uncertainty -= frequency * log2(frequency);
    }
  }

  return uncertainty;
}

//------------------------------------------------------------------------------

double
sooth_predictor_surprise(SoothPredictor * predictor, uint32_t id, uint32_t event)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);

  if (context == NULL || context->count == 0)
  {
    return -1;
  }

  SoothStatistic * statistic = sooth_predictor_find_statistic(context, event);

  if (statistic == NULL || statistic->count == 0)
  {
    return -1;
  }

  double frequency = (double)statistic->count / (double)context->count;
  return -log2(frequency);
}

//------------------------------------------------------------------------------

double
sooth_predictor_frequency(SoothPredictor * predictor, uint32_t id, uint32_t event)
{
  SoothContext * context = sooth_predictor_find_context(predictor, id);

  if (context == NULL || context->count == 0)
  {
    return 0.0;
  }

  SoothStatistic * statistic = sooth_predictor_find_statistic(context, event);

  if (statistic == NULL || statistic->count == 0)
  {
    return 0.0;
  }

  return (double)statistic->count / (double)context->count;
}

//==============================================================================
