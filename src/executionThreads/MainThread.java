package executionThreads;

import constants.Constants;

public class MainThread{	
	
	public static void main(String[] args) {
		HourGlass g1 = new HourGlass();
		new TimerThread(g1);
		new PopupThread(g1);	
	}
}

class HourGlass{
	static boolean starterNode = true;
	Timer timer = new Timer();
	PopupWindow popup = new PopupWindow();
	
	//starter node
	public synchronized void startTimer() {
		if(!starterNode) {	//if other node has to execute
			try {
				wait();
				System.out.println("Waiting for next node to finish");
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}
		timer.startTimer();
		System.out.println("Finished timer. Shifting execution to other thread.");
		starterNode = false;
		notifyAll();
	}
	
	//other node
	public synchronized void openWindow() {
		if(starterNode) {	//if starter node has to execute
			try {
				System.out.println("Waiting for starter node to finish");
				wait();
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}
		System.out.println("Going to open window");
		popup.openWindow();
		try {
			Thread.sleep(Constants.buttonDisableDuration);
		} catch (InterruptedException e) {
			e.printStackTrace();
		}
		Constants.disabledButton.setEnabled(true);
		while(Constants.continueTimer == false) {
			try {
				Thread.sleep(1000);
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}
		popup.closeWindow();
		Constants.continueTimer = false;	
		starterNode = true;
		notifyAll();
	}
}

class TimerThread implements Runnable {
	HourGlass g;
	
	public TimerThread(HourGlass ins) {
		this.g = ins;
		new Thread(this, "TimerThread").start();
	}
	
	public void run() {
		while(true) {
			g.startTimer();
		}
	}
}

class PopupThread implements Runnable {
	HourGlass g;
	
	public PopupThread(HourGlass ins) {
		this.g = ins;
		new Thread(this, "PopupThread").start();
	}
	public void run() {
		while(true) {
			g.openWindow();
		}
	}
}