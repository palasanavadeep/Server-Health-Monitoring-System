"use client";

import React, { createContext, useContext, useEffect, useState } from 'react';

type ThemeType = 'cyber' | 'light';

interface ThemeDetails {
    name: string;
    description: string;
    class: string;
}

interface ThemeContextType {
    currentTheme: ThemeType;
    themes: Record<ThemeType, ThemeDetails>;
    switchTheme: (themeName: ThemeType) => void;
    theme: ThemeDetails;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export function useTheme() {
    const context = useContext(ThemeContext);
    if (!context) {
        throw new Error('useTheme must be used within a ThemeProvider');
    }
    return context;
}

const themes: Record<ThemeType, ThemeDetails> = {
    cyber: {
        name: 'Cyber (Dark)',
        description: 'Sleek dark zinc layout with cyan and electric emerald accents',
        class: 'theme-cyber'
    },
    light: {
        name: 'Light (Modern)',
        description: 'Clean bright layout for high-glare setups',
        class: 'theme-light'
    }
};

export function ThemeProvider({ children }: { children: React.ReactNode }) {
    const [currentTheme, setCurrentTheme] = useState<ThemeType>('cyber');
    const [mounted, setMounted] = useState(false);

    useEffect(() => {
        const savedTheme = localStorage.getItem('app-theme') as ThemeType;
        if (savedTheme && themes[savedTheme]) {
            setCurrentTheme(savedTheme);
        }
        setMounted(true);
    }, []);

    useEffect(() => {
        if (!mounted) return;
        
        const root = document.documentElement;
        Object.values(themes).forEach(theme => {
            root.classList.remove(theme.class);
        });

        root.classList.add(themes[currentTheme].class);
        localStorage.setItem('app-theme', currentTheme);
    }, [currentTheme, mounted]);

    const switchTheme = (themeName: ThemeType) => {
        if (themes[themeName]) {
            setCurrentTheme(themeName);
        }
    };

    const value = {
        currentTheme,
        themes,
        switchTheme,
        theme: themes[currentTheme]
    };

    return (
        <ThemeContext.Provider value={value}>
            {children}
        </ThemeContext.Provider>
    );
}
