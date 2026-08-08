"use client";

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useTheme } from '@/contexts/theme-context';
import { MetricStats } from '@/lib/api';

interface ApiHitsChartProps {
    stats: MetricStats | null;
}

export function ApiHitsChart({ stats }: ApiHitsChartProps) {
    const { currentTheme } = useTheme();
    const isLight = currentTheme === 'light';

    const isEmpty = !stats || (stats.totalHits === 0 && stats.successHits === 0 && stats.errorHits === 0);

    const data = [
        { name: 'Total Hits', value: stats?.totalHits ?? 0, color: 'url(#totalGlow)' },
        { name: 'Success', value: stats?.successHits ?? 0, color: 'url(#successGlow)' },
        { name: 'Errors', value: stats?.errorHits ?? 0, color: 'url(#errorGlow)' },
    ];

    return (
        <Card className="flex flex-col h-[400px]">
            <CardHeader>
                <CardTitle>API Traffic Summary</CardTitle>
                <CardDescription>Total, success and error hit counts</CardDescription>
            </CardHeader>
            <CardContent className="flex-1 flex flex-col justify-center min-h-0">
                {isEmpty ? (
                    <div className="flex flex-col items-center justify-center h-48 text-center">
                        <p className="text-sm font-medium text-foreground">No traffic data available yet</p>
                        <span className="text-xs text-muted-foreground mt-1">Data will appear once API requests are ingested</span>
                    </div>
                ) : (
                    <div className="w-full h-full min-h-[220px]">
                        <ResponsiveContainer width="100%" height="100%">
                            <BarChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 5 }}>
                                <defs>
                                    <linearGradient id="totalGlow" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="0%" stopColor="#3b82f6" stopOpacity={0.8} />
                                        <stop offset="100%" stopColor="#06b6d4" stopOpacity={0.3} />
                                    </linearGradient>
                                    <linearGradient id="successGlow" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="0%" stopColor="#10b981" stopOpacity={0.8} />
                                        <stop offset="100%" stopColor="#059669" stopOpacity={0.3} />
                                    </linearGradient>
                                    <linearGradient id="errorGlow" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="0%" stopColor="#f43f5e" stopOpacity={0.8} />
                                        <stop offset="100%" stopColor="#e11d48" stopOpacity={0.3} />
                                    </linearGradient>
                                </defs>
                                <XAxis 
                                    dataKey="name" 
                                    stroke={isLight ? '#64748b' : '#94a3b8'} 
                                    fontSize={12}
                                    tickLine={false}
                                    axisLine={false}
                                />
                                <YAxis 
                                    stroke={isLight ? '#64748b' : '#94a3b8'} 
                                    fontSize={12}
                                    tickLine={false}
                                    axisLine={false}
                                />
                                <Tooltip
                                    cursor={{ fill: isLight ? 'rgba(0,0,0,0.02)' : 'rgba(255,255,255,0.02)', radius: 4 }}
                                    content={({ active, payload }) => {
                                        if (active && payload && payload.length) {
                                            const item = payload[0];
                                            return (
                                                <div className="glass-panel p-2.5 rounded-lg border border-border-color shadow-lg text-sm text-foreground">
                                                    <span className="font-semibold">{item.name}</span>
                                                    <p className="text-xs text-muted-foreground mt-0.5">
                                                        Hits: <span className="font-mono text-foreground font-semibold ml-1">{Number(item.value).toLocaleString()}</span>
                                                    </p>
                                                </div>
                                            );
                                        }
                                        return null;
                                    }}
                                />
                                <Bar 
                                    dataKey="value" 
                                    radius={[6, 6, 0, 0]}
                                    maxBarSize={60}
                                >
                                    {data.map((entry, index) => (
                                        <Cell key={`cell-${index}`} fill={entry.color} />
                                    ))}
                                </Bar>
                            </BarChart>
                        </ResponsiveContainer>
                    </div>
                )}
            </CardContent>
        </Card>
    );
}
