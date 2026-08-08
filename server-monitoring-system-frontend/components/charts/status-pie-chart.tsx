"use client";

import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useTheme } from '@/contexts/theme-context';

interface StatusDistributionChartProps {
    data: {
        labels: string[];
        values: number[];
    } | null;
}

export function StatusDistributionChart({ data }: StatusDistributionChartProps) {
    const { currentTheme } = useTheme();
    const isLight = currentTheme === 'light';

    const values = data?.values ?? [];
    const isEmpty = !values.length || values.every((v) => v === 0);

    const chartData = [
        { name: 'Success (2xx)', value: values[0] ?? 0, color: '#10b981' },
        { name: 'Errors (4xx/5xx)', value: values[1] ?? 0, color: '#f43f5e' },
    ];

    const totalRequests = values.reduce((sum, current) => sum + current, 0);

    return (
        <Card className="flex flex-col h-[400px]">
            <CardHeader>
                <CardTitle>Status Code Distribution</CardTitle>
                <CardDescription>HTTP status code breakdown</CardDescription>
            </CardHeader>
            <CardContent className="flex-1 flex flex-col justify-center min-h-0">
                {isEmpty ? (
                    <div className="flex flex-col items-center justify-center h-48 text-center">
                        <p className="text-sm font-medium text-foreground">No request data available yet</p>
                        <span className="text-xs text-muted-foreground mt-1">Data will appear once API requests are ingested</span>
                    </div>
                ) : (
                    <div className="w-full h-full min-h-[220px] relative">
                        <ResponsiveContainer width="100%" height="100%">
                            <PieChart>
                                <Pie
                                    data={chartData}
                                    cx="50%"
                                    cy="50%"
                                    innerRadius={70}
                                    outerRadius={90}
                                    paddingAngle={3}
                                    dataKey="value"
                                    animationDuration={500}
                                >
                                    {chartData.map((entry, index) => (
                                        <Cell key={`cell-${index}`} fill={entry.color} />
                                    ))}
                                </Pie>
                                <Tooltip
                                    content={({ active, payload }) => {
                                        if (active && payload && payload.length) {
                                            const item = payload[0];
                                            const percentage = totalRequests > 0 
                                                ? ((item.value as number) / totalRequests * 100).toFixed(1) 
                                                : '0.0';
                                            return (
                                                <div className="glass-panel p-2.5 rounded-lg border border-border-color shadow-lg text-sm text-foreground">
                                                    <div className="flex items-center gap-2">
                                                        <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: item.payload.color }} />
                                                        <span className="font-semibold">{item.name}</span>
                                                    </div>
                                                    <p className="text-xs text-muted-foreground mt-1">
                                                        Requests: <span className="font-mono text-foreground font-semibold ml-1">{Number(item.value).toLocaleString()}</span>
                                                        <span className="text-muted-foreground font-medium ml-1">({percentage}%)</span>
                                                    </p>
                                                </div>
                                            );
                                        }
                                        return null;
                                    }}
                                />
                                <text
                                    x="50%"
                                    y="50%"
                                    textAnchor="middle"
                                    dominantBaseline="middle"
                                    className="text-foreground"
                                >
                                    <tspan x="50%" dy="-6" className="text-2xl font-bold fill-current">
                                        {totalRequests.toLocaleString()}
                                    </tspan>
                                    <tspan x="50%" dy="20" className="text-xs fill-current text-muted-foreground font-medium">
                                        Total Requests
                                    </tspan>
                                </text>
                            </PieChart>
                        </ResponsiveContainer>
                    </div>
                )}
            </CardContent>
        </Card>
    );
}
export default StatusDistributionChart;
