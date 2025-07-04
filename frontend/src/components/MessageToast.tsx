import {useEffect, useState} from 'react';

interface MessageToastProps {
    message: string;
    type: 'success' | 'error';
}

export function MessageToast({ message, type }: MessageToastProps) {
    const [show, setShow] = useState(true);
    
    useEffect(() => {
        const timer = setTimeout(() => {
            setShow(false);
        }, 3000);
        
        return () => clearTimeout(timer);
    }, []); 
    
    return (
        show && (
            <span className={`${type === 'success' ? 'text-green-500' : 'text-red-500'}`}>{message}</span>
        )
    );
}