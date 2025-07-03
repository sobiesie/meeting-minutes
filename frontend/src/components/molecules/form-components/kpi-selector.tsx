'use client';

import * as React from 'react';
import { Badge } from '@/components/atoms-shadcnui/ui/badge';
import { X } from 'lucide-react';
import { cn } from '@/lib/utils-shadcnui';
import { useTranslations } from 'next-intl';

interface KpiSelectorProps {
  selectedKpis: string[];
  onKpiChange: (kpis: string[]) => void;
  className?: string;
}

export const getKpis = (t: (key: string) => string): string[] => {
  return [
    t('performanceMetrics.yield'),
    t('performanceMetrics.overallEquipmentEfficiency'),
    t('performanceMetrics.labourEfficiency'),
    t('performanceMetrics.cycleTime'),
    t('performanceMetrics.throughput'),
    t('performanceMetrics.energyCostReduction'),
    t('performanceMetrics.wasteReduction'),
    t('performanceMetrics.assetTurnover'),
    t('performanceMetrics.capacityUtilisation'),
    t('performanceMetrics.downtime'),
  ];
};

// const PREDEFINED_KPIS = [
//   'Yield',
//   'Overall Equipment Efficiency',
//   'Labour Efficiency',
//   'Cycle Time',
//   'Throughput',
//   'Energy Cost Reduction',
//   'Waste Reduction',
//   'Asset Turnover',
//   'Capacity Utilisation',
//   'Downtime',
// ];

export function KpiSelector({
  selectedKpis,
  onKpiChange,
  className,
}: KpiSelectorProps) {
  const t = useTranslations('Dashboard');

  // Reset KPI selection when component mounts or selectedKpis changes
  React.useEffect(() => {
    if (selectedKpis.length > 0) {
      onKpiChange([]);
    }
  }, []); // Empty dependency array means this runs once on mount

  const handleKpiSelect = (kpi: string) => {
    if (selectedKpis.includes(kpi)) {
      onKpiChange(selectedKpis.filter((k) => k !== kpi));
    } else {
      onKpiChange([...selectedKpis, kpi]);
    }
  };

  return (
    <div className={cn('space-y-4', className)}>
      <div className="flex flex-wrap gap-2">
        {selectedKpis.map((kpi) => (
          <Badge
            key={kpi}
            variant="secondary"
            className="flex items-center gap-1"
          >
            {kpi}
            <button
              type="button"
              onClick={() => handleKpiSelect(kpi)}
              className="ml-1 hover:text-destructive"
            >
              <X className="h-3 w-3" />
            </button>
          </Badge>
        ))}
      </div>
      <div className="grid grid-cols-2 gap-2">
        {getKpis(t).map((kpi) => (
          <button
            key={kpi}
            type="button"
            onClick={() => handleKpiSelect(kpi)}
            className={cn(
              'rounded-md border p-2 text-sm transition-colors break-words',
              selectedKpis.includes(kpi)
                ? 'border-primary bg-primary/10 text-primary'
                : 'border-border hover:bg-accent'
            )}
          >
            {kpi}
          </button>
        ))}
      </div>
    </div>
  );
}
